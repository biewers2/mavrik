use std::sync::mpsc::Sender;
use log::info;
use crate::events::{MavrikEvent, Task};

pub fn listen_for_tcp_connections(event_tx: Sender<MavrikEvent>) -> Result<(), anyhow::Error> {
    let task = Task {
        queue: "default".to_owned(),
        definition: "Test".to_owned(),
        args: "".to_owned()
    };
    for _ in 0..25 {
        event_tx.send(MavrikEvent::Task(task.clone()))?;
    }
    
    info!("Starting TCP listener");
    Ok(())
}
