use crate::events::{MavrikEvent, MavrikRequest, MavrikResponse, Task};
use crate::io::{read_deserialized_async, write_serialized_async};
use async_std::net::{TcpListener, TcpStream};
use libc::{getppid, kill, SIGUSR1};
use log::{debug, info, warn};
use signal_hook::consts::SIGTERM;
use std::sync::mpsc::Sender;
use tokio::task::JoinSet;
use crate::runtime::async_runtime;

pub struct TcpServerOptions {
    pub host: String,
    pub port: u16,
    pub signal_parent_ready: bool
}

pub fn listen_for_tcp_connections(options: TcpServerOptions, event_tx: Sender<MavrikEvent>) -> Result<(), anyhow::Error> {
    info!("Starting TCP listener");
    
    let address = format!("{}:{}", options.host, options.port);
    
    async_runtime().block_on(async move {
        let listener = TcpListener::bind(address).await?;
        if options.signal_parent_ready {
            signal_ready_to_parent_process();
        }
        
        let mut connections = JoinSet::new();

        info!("Listening for incoming connections");
        while let Ok((stream, addr)) = listener.accept().await {
            debug!("Accepted connection from {addr:?}");
            connections.spawn(handle_client(stream, event_tx.clone()));
        }
        
        for joined_conn in connections.join_all().await {
            joined_conn?;
        }
        Ok(())
    })
}

fn signal_ready_to_parent_process() {
    match unsafe { kill(getppid(), SIGUSR1) } {
        0 => info!("Successfully signalled ready to parent process"),
        _ => warn!("Failed to send ready signal to parent process")
    }
}

pub async fn handle_client(mut stream: TcpStream, event_tx: Sender<MavrikEvent>) -> Result<(), anyhow::Error> {
    loop {
        let response = match read_deserialized_async(&mut stream).await? {
            MavrikRequest::NewTask(new_task) => {
                let task = Task::from(new_task);
                let task_id = task.id.clone();
                event_tx.send(MavrikEvent::NewTask(task))?;
                MavrikResponse::NewTaskId(task_id)
            },

            MavrikRequest::Terminate => {
                event_tx.send(MavrikEvent::Signal(SIGTERM))?;
                MavrikResponse::Terminated(true)
            },
        };

        debug!("Sending response {response:?} over TCP");
        write_serialized_async(&mut stream, &response).await?;
    }
}
