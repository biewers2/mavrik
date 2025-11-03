use crate::messaging::TaskId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoreState {
    pub tasks: Vec<StoredTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredTask {
    pub id: TaskId,
    pub status: StoredTaskStatus,
    pub definition: String,
    pub args: String,
    pub kwargs: String
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StoredTaskStatus {
    Enqueued,
    Processing,
    Completed,
    Retrying,
    Failed
}
