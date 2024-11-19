use crate::messaging::{MavrikRequest, MavrikResponse, Task, TaskId};
use crate::service::MavrikService;
use crate::store::{PullStore, PushStore};
use crate::tcp::util::{read_deserialized, write_serialized};
use anyhow::Context;
use log::trace;
use tokio::net::TcpStream;

pub struct TcpClientHandler<Store> {
    stream: TcpStream,
    store: Store,
}

impl<Store> TcpClientHandler<Store> {
    pub fn new(stream: TcpStream, store: Store) -> Self {
        Self { stream, store }
    }
}

impl<Store> MavrikService for TcpClientHandler<Store>
where
    Store: PushStore<Id = TaskId, Error = anyhow::Error>
        + PullStore<Id = TaskId, Error = anyhow::Error>
        + Send
        + Sync
        + 'static,
{
    type TaskOutput = Result<MavrikRequest, anyhow::Error>;
    type Message = ();

    async fn poll_task(&mut self) -> Self::TaskOutput {
        read_deserialized(&mut self.stream)
            .await
            .context("receiving Mavrik request over TCP")
    }

    async fn on_task_ready(&mut self, request: Self::TaskOutput) -> Result<(), anyhow::Error> {
        let response = match request? {
            MavrikRequest::NewTask(new_task) => {
                let task = Task::from(new_task);
                let task_id = self.store.push(task).await.context("store push failed")?;
                MavrikResponse::NewTaskId(task_id)
            }

            MavrikRequest::AwaitTask { task_id } => {
                let task_result = self
                    .store
                    .pull(task_id)
                    .await
                    .context("store pull failed")?;
                MavrikResponse::CompletedTask(task_result)
            }
        };

        trace!(response:?; "Sending response over TCP");
        write_serialized(&mut self.stream, &response)
            .await
            .context("failed to send Mavrik response over TCP")?;
        Ok(())
    }
}
