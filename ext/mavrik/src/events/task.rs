use crate::rb::class_mavrik_error;
use log::kv::{ToValue, Value};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct TaskId(pub [u8; 20]);

#[derive(Debug, Serialize, Deserialize)]
pub struct NewTask {
    pub queue: String,
    pub ctx: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub queue: String,
    pub ctx: String
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

impl Display for TaskId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Serde JSON doesn't support u128 :( so we use two u64s.
        
        let mut time_0_buf = [0u8; 8];
        let mut time_1_buf = [0u8; 8];
        let mut count_buf = [0u8; 4];
        
        time_0_buf.clone_from_slice(&self.0[..8]);
        time_1_buf.clone_from_slice(&self.0[8..16]);
        count_buf.clone_from_slice(&self.0[16..]);
        
        let time_0: u64 = u64::from_be_bytes(time_0_buf);
        let time_1: u64 = u64::from_be_bytes(time_1_buf);
        let count: u32 = u32::from_be_bytes(count_buf);
        
        write!(f, "{}_{}-{}", time_0, time_1, count)
    }
}

impl ToValue for TaskId {
    fn to_value(&self) -> Value {
        Value::from_display(self)
    }
}

impl From<NewTask> for Task {
    fn from(value: NewTask) -> Self {
        let NewTask { queue, ctx } = value;
        Self { queue, ctx }
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
