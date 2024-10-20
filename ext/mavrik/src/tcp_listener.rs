use crate::events::{MavrikEvent, MavrikRequest, MavrikResponse, Task};
use crate::io::{read_deserialized_async, write_serialized_async};
use async_std::net::{TcpListener, TcpStream};
use futures::{AsyncWriteExt, TryFutureExt};
use libc::{getppid, kill, SIGTERM, SIGUSR1};
use log::{debug, info, warn};
use tokio::select;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;

pub struct TcpServerOptions {
    pub host: String,
    pub port: u16,
    pub signal_parent_ready: bool
}

pub struct TcpServerParams {
    pub event_tx: mpsc::Sender<MavrikEvent>,
    pub term_rx: oneshot::Receiver<()>
}

pub async fn listen_for_tcp_connections(options: TcpServerOptions, params: TcpServerParams) -> Result<(), anyhow::Error>
{
    info!("Starting TCP listener");
    let TcpServerOptions {
        host,
        port,
        signal_parent_ready
    } = options;
    let TcpServerParams { 
        event_tx, 
        mut term_rx 
    } = params;
    
    let listener = TcpListener::bind(format!("{host}:{port}")).await?;
    debug!("TCP listener accepting connections on {host}:{port}");
    
    if signal_parent_ready {
        signal_ready_to_parent_process();
    }

    info!("Listening for incoming connections");
    let term_rx = &mut term_rx;
    let mut connections = JoinSet::new();
    let mut conn_term_txs = vec![];
    loop {
        select! {
            conn = listener.accept() => {
                let (stream, addr) = conn?;
                let (conn_term_tx, conn_term_rx) = oneshot::channel();
                connections.spawn(handle_client(stream, event_tx.clone(), conn_term_rx));
                conn_term_txs.push(conn_term_tx);
                debug!("Accepted connection from {addr:?}");
            },
            result = term_rx.into_future() => {
                debug!("[TCP] Received term");
                result?;
                for conn_term_tx in conn_term_txs {
                    let _ = conn_term_tx.send(());
                }
                break
            }
        }
    }
    
    info!("Terminating all TCP connections");
    while let Some(result) = connections.join_next().await {
        result??;
    }
    
    info!("TCP listener stopped");
    Ok(())
}

fn signal_ready_to_parent_process() {
    match unsafe { kill(getppid(), SIGUSR1) } {
        0 => info!("Successfully signalled ready to parent process"),
        _ => warn!("Failed to send ready signal to parent process")
    }
}

pub async fn handle_client(mut stream: TcpStream, event_tx: mpsc::Sender<MavrikEvent>, mut term_rx: oneshot::Receiver<()>) -> Result<(), anyhow::Error> {
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
                debug!("[TCP][Client] Received term");
                result?;
                stream.close().await?;
                break Ok(())
            }
        }
    }
}
