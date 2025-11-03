use crate::messaging::task_id::TaskId;
use crate::messaging::NewTask;
use crate::store::StoreState;
use serde::{Deserialize, Serialize};

/// A request made from a TCP client to the TCP listener service ("TCP").
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MavrikRequest {
    /// A new task being submitted.
    NewTask { queue: String, payload: NewTask },

    /// Get the state of the storage container.
    GetStoreState,
}

/// A response given to a TCP client from the TCP listener service ("TCP").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum MavrikResponse {
    /// The response for submitting a new task.
    /// Contains the created ID of the task submitted.
    NewTaskId(TaskId),

    /// The state of the storage container.
    StoreState(StoreState),
}
