use crate::events::{NewTask, Task};
use serde::{Deserialize, Serialize};
use std::ffi::c_int;
use crate::task_executor::ThreadId;

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
    ThreadReady(ThreadId),
    Signal(c_int),
    NewTask(Task)
}
