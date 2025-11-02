use crate::io::{read_object, write_object};
use crate::messaging::{MavrikRequest, MavrikResponse, Task, TaskId};
use crate::service::ServiceTask;
use crate::store::{PullStore, PushStore, QueryStore};
use anyhow::Context;
use log::trace;
use tokio::net::TcpStream;

pub struct TcpClientHandler<Store> {
    stream: TcpStream,
    store: Store,
}

impl<Store> TcpClientHandler<Store>
where
    Store: PushStore<Id = TaskId, Error = anyhow::Error> 
        + PullStore<Id = TaskId, Error = anyhow::Error>
        + QueryStore<Error = anyhow::Error>
        + Clone + Send + Sync + 'static,
    
{
    pub fn new(stream: TcpStream, store: Store) -> Self {
        Self { stream, store }
    }
}

impl<Store> ServiceTask for TcpClientHandler<Store>
where
    Store: PushStore<Id = TaskId, Error = anyhow::Error>
        + PullStore<Id = TaskId, Error = anyhow::Error>
        + QueryStore<Error = anyhow::Error>
        + Clone + Send + Sync + 'static,
{
    type TaskOutput = Result<MavrikRequest, anyhow::Error>;

    async fn poll_task(&mut self) -> Self::TaskOutput {
        read_object(&mut self.stream)
            .await
            .context("receiving Mavrik request over TCP failed")
    }

    async fn on_task_ready(&mut self, request: Self::TaskOutput) -> Result<(), anyhow::Error> {
        match request? {
            MavrikRequest::NewTask(new_task) => {
                let task = Task::from(new_task);
                let task_id = self.store.push(task).await.context("store push failed")?;
                let response = MavrikResponse::NewTaskId(task_id);

                trace!(response:?; "Sending response over TCP");
                write_object(&mut self.stream, &response)
                    .await
                    .context("sending new task ID over TCP failed")?;
            },

            MavrikRequest::GetStoreState => {
                let state = self.store.state().await?;
                let response = MavrikResponse::StoreState(state);
                write_object(&mut self.stream, &response)
                    .await
                    .context("sending state over TCP failed")?;
            }
        };
        Ok(())
    }
}

