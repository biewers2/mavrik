use std::ffi::c_int;
use crate::task_executor::ReadyThread;

#[derive(Debug)]
pub enum MavrikEvent {
    ReadyThread(ReadyThread),
    Task(Task),
    Signal(c_int)
}

#[derive(Debug, Clone)]
pub struct Task {
    pub queue: String,
    pub definition: String,
    pub args: String
}
