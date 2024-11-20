use crate::messaging::task_id::TaskId;
use crate::messaging::{NewTask, TaskResult};
use serde::{Deserialize, Serialize};
use crate::store::StoreState;

/// A request made from a TCP client to the TCP listener service ("TCP").
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MavrikRequest {
    /// A new task being submitted.
    NewTask(NewTask),

    /// Wait for a task to finish and return its result.
    AwaitTask { task_id: TaskId },
    
    GetStoreState
}

/// A response given to a TCP client from the TCP listener service ("TCP").
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum MavrikResponse {
    /// The response for submitting a new task.
    /// Contains the created ID of the task submitted.
    NewTaskId(TaskId),

    /// The result of waiting for a task to complete.
    CompletedTask(TaskResult),
    
    StoreState(StoreState),
}
