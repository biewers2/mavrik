use std::future::IntoFuture;
use libc::SIGTERM;
use log::debug;
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::{mpsc, oneshot};
use crate::events::{MavrikEvent, MavrikRequest, MavrikResponse, Task};
use crate::io::util::{read_deserialized_async, write_serialized_async};

pub struct HandleTcpStreamParams {
    pub event_tx: mpsc::Sender<MavrikEvent>,
    pub term_rx: oneshot::Receiver<()>
}

pub async fn handle_tcp_stream(mut stream: TcpStream, params: HandleTcpStreamParams) -> Result<(), anyhow::Error> {
    let HandleTcpStreamParams {
        event_tx,
        mut term_rx
    } = params;
    
    let term_rx = &mut term_rx;
    loop {
        select! {
            response = read_deserialized_async(&mut stream) => {
                let response = match response? {
                    MavrikRequest::NewTask(new_task) => {
                        let task = Task::from(new_task);
                        let task_id = task.id.clone();
                        event_tx.send(MavrikEvent::NewTask(task)).await?;
                        MavrikResponse::NewTaskId(task_id)
                    },

                    MavrikRequest::Terminate => {
                        event_tx.send(MavrikEvent::Signal(SIGTERM)).await?;
                        MavrikResponse::Terminated(true)
                    },
                };
                
                debug!("Sending response {response:?} over TCP");
                write_serialized_async(&mut stream, &response).await?;
            },
            result = term_rx.into_future() => {
                result?;
                debug!("[TCP Stream] Received term");
                
                break;
            }
        }
    }

    Ok(())
}