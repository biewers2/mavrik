use crate::events::{AwaitedTask, NewTask, Task};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MavrikRequest {
    NewTask(NewTask),
    Terminate
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum MavrikResponse {
    NewTaskId(String),
    Terminated(bool)
}

#[derive(Debug)]
pub enum MavrikEvent {
    General(GeneralEvent),
    Exe(ExeEvent),
    Tcp(TcpEvent),
    Sig(SigEvent)
}

#[derive(Debug)]
pub enum GeneralEvent {
    Terminate
}

#[derive(Debug)]
pub enum ExeEvent {
    NewTask(Task),
    
    AwaitTask {
        task_id: String,
        value_tx: oneshot::Sender<AwaitedTask>
    }
}

#[derive(Debug)]
pub enum TcpEvent {}

#[derive(Debug)]
pub enum SigEvent {}
