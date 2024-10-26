use anyhow::Context;
use crate::events::{ExeEvent, GeneralEvent, MavrikEvent, MavrikRequest, MavrikResponse, Task};
use crate::service::Service;
use crate::tcp::util::{read_deserialized, write_serialized};
use log::{error, info, trace};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

/// Params for creating a TCP client handler.
pub struct TcpClientHandlerParams {
    /// Where to send events to.
    pub event_tx: mpsc::Sender<MavrikEvent>
}

/// Handles TCP client connections and requests.
pub struct TcpClientHandler {
    /// The TCP stream to read from/write to.
    stream: TcpStream,
    
    /// Where to send events to.
    event_tx: mpsc::Sender<MavrikEvent>
}

impl TcpClientHandler {
    /// Create a new TCP clietn handler from an accepted client stream.
    pub fn new(stream: TcpStream, params: TcpClientHandlerParams) -> Self {
        let TcpClientHandlerParams { event_tx } = params;
        Self { stream, event_tx }
    }
}

impl Service for TcpClientHandler {
    type TaskOutput = Result<MavrikRequest, anyhow::Error>;
    type Message = ();

    // Read and deserialize requests from the client.
    async fn poll_task(&mut self) -> Self::TaskOutput {
        read_deserialized(&mut self.stream).await.context("receiving Mavrik request over TCP")
    }

    // Handle the request and provide the appropriate response.
    async fn on_task_ready(&mut self, request: Self::TaskOutput) -> Result<(), anyhow::Error> {
        let response = match request? {
            
            // New tasks are send to the task executor to be executed.
            // The generated task ID is sent back to the client.
            MavrikRequest::NewTask(new_task) => {
                let (value_tx, value_rx) = oneshot::channel();
                
                let task = Task::from(new_task);
                let event = MavrikEvent::Exe(ExeEvent::NewTask { task, value_tx });
                self.event_tx.send(event).await.context("sending new task event from client handler")?;
                
                let task_id = value_rx.await.context("awaiting task ID from oneshot channel")?;
                MavrikResponse::NewTaskId(task_id)
            },
            
            // Await for a task's completion by async receiving the result over a oneshot channel.
            MavrikRequest::AwaitTask { task_id } => {
                let (value_tx, value_rx) = oneshot::channel();
                
                let event = MavrikEvent::Exe(ExeEvent::AwaitTask { task_id, value_tx });
                self.event_tx.send(event).await.context("sending await task event from client handler")?;
                
                let task_result = value_rx.await.context("awaiting task result from oneshot channel")?;
                MavrikResponse::CompletedTask(task_result)
            },

            // Tell the event loop to terminate all services when the client requests so.
            MavrikRequest::Terminate => {
                let event = MavrikEvent::General(GeneralEvent::Terminate);
                let result = self.event_tx.send(event).await.context("sending terminate event from client");
                
                match result {
                    Ok(_) => {
                        info!("Successfully fulfilled termination request from client");
                        MavrikResponse::Terminated(true)
                    },
                    Err(e) => {
                        error!(e:?; "Failed to fulfill termination request from client");
                        MavrikResponse::Terminated(false)
                    }
                }
            },
        };

        trace!(response:?; "Sending response over TCP");
        write_serialized(&mut self.stream, &response).await.context("failed to send Mavrik response over TCP")?;
        Ok(())
    }
}
