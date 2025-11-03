use crate::rb::util::class_mavrik_error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct NewTask {
    pub definition: String,
    pub args: String, // Serialized
    pub kwargs: String // Serialized
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Task {
    pub definition: String,
    pub args: String, // Serialized
    pub kwargs: String // Serialized
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskResult {
    Success {
        result: String // Serialized
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
