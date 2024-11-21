use crate::rb::class_mavrik_error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NewTask {
    pub queue: String,
    pub definition: String,
    pub args: String, // Serialized
    pub kwargs: String // Serialized
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub queue: String,
    pub definition: String,
    pub args: String, // Serialized
    pub kwargs: String // Serialized
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskResult {
    Success {
        result: serde_json::Value
    },
    Failure {
        class: String,
        message: String,
        backtrace: Vec<String>
    }
}

impl From<NewTask> for Task {
    fn from(value: NewTask) -> Self {
        Self {
            queue: value.queue,
            definition: value.definition,
            args: value.args,
            kwargs: value.kwargs,
        }
    }
}

impl From<anyhow::Error> for TaskResult {
    fn from(value: anyhow::Error) -> Self {
        TaskResult::Failure {
            class: class_mavrik_error().to_string(),
            message: format!("{value}"),
            backtrace: vec![]
        }
    }
}
