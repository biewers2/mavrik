use crate::events::MavrikEvent;
use log::info;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::Sender;
use serde::{Deserialize, Serialize};
use crate::rb::SubmittedTask;

pub trait SerialEvent<'de>: Serialize + Deserialize<'de> {}

pub fn listen_for_tcp_connections(event_tx: Sender<MavrikEvent>) -> Result<(), anyhow::Error> {
    info!("Starting TCP listener");

    let listener = TcpListener::bind("127.0.0.1:3009")?;
    for stream in listener.incoming() {
        handle_client(stream?, event_tx.clone())?;
    }

    Ok(())
}

pub fn handle_client(stream: TcpStream, event_tx: Sender<MavrikEvent>) -> Result<(), anyhow::Error> {
    loop {
        let submitted_task = serde_json::from_reader::<_, SubmittedTask>(&stream)?;
        let task_event = MavrikEvent::Task(submitted_task.into());
        event_tx.send(task_event)?;
    }
    Ok(())
}
