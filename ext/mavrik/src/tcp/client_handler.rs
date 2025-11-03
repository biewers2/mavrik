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
        + Clone
        + Send
        + Sync
        + 'static,
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
        + Clone
        + Send
        + Sync
        + 'static,
{
    type ReadyTask = Result<MavrikRequest, anyhow::Error>;

    async fn poll_task(&mut self) -> Self::ReadyTask {
        read_object(&mut self.stream)
            .await
            .context("receiving Mavrik request over TCP failed")
    }

    async fn on_task_ready(&mut self, request: Self::ReadyTask) -> Result<(), anyhow::Error> {
        let response = match request? {
            MavrikRequest::NewTask { queue, payload } => {
                let task = Task::from(payload);
                let task_id = self
                    .store
                    .push(&queue, task)
                    .await
                    .context("store push failed")?;
                MavrikResponse::NewTaskId(task_id)
            }

            MavrikRequest::GetStoreState => {
                let state = self.store.state().await?;
                MavrikResponse::StoreState(state)
            }
        };

        trace!(response:?; "Sending response over TCP");
        write_object(&mut self.stream, &response)
            .await
            .context("sending response over TCP failed")
    }
}
