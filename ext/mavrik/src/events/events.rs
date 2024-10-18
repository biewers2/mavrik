use crate::events::{NewTask, Task};
use serde::{Deserialize, Serialize};
use std::ffi::c_int;
use std::sync::mpsc::Sender;

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
    ReadyThread(ReadyThread),
    Signal(c_int),
    NewTask(Task)
}

#[derive(Debug)]
pub struct ReadyThread {
    pub task_tx: Sender<Task>
}
