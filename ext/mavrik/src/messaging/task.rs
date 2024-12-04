use anyhow::anyhow;
use magnus::RHash;
use crate::rb::util::{class_mavrik_error, mavrik_error, MRHash};
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

impl TryFrom<RHash> for TaskResult {
    type Error = magnus::Error;

    fn try_from(h: RHash) -> Result<Self, Self::Error> {
        let h = MRHash(h);
        
        let variant = h.try_fetch_sym::<String>("type")?;
        match variant.as_str() {
            "success" => {
                Ok(Self::Success {
                    result: h.try_fetch_sym("result")?,
                })
            },
            
            "failure" => {
                Ok(Self::Failure {
                    class: h.try_fetch_sym("class")?,
                    message: h.try_fetch_sym("message")?,
                    backtrace: h.try_fetch_sym("backtrace")?,
                })
            }
            
            _ => Err(mavrik_error(anyhow!("unsupported request type: {}", variant))),
        }
    }
}
