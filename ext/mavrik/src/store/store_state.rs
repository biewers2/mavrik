use std::collections::HashMap;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use crate::messaging::{Task, TaskId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoreState {
    pub tasks: Vec<StoredTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredTask {
    pub id: TaskId,
    pub status: StoredTaskStatus,
    pub context: StoredTaskContext
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StoredTaskStatus {
    Enqueued,
    Processing,
    Completed,
    Retrying,
    Failed
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredTaskContext {
    pub definition: String,
    pub args: Vec<String>,
    pub kwargs: HashMap<String, serde_json::Value>,
}

impl TryFrom<&Task> for StoredTaskContext {
    type Error = anyhow::Error;

    fn try_from(value: &Task) -> Result<Self, Self::Error> {
        let ctx: HashMap<String, String> = serde_json::from_str(&value.ctx)?;
        let definition = ctx.get("def").ok_or(anyhow!("Task definition missing"))?.to_owned();
        let args_str = ctx.get("args").ok_or(anyhow!("Args missing"))?;
        let args: Vec<String> = serde_json::from_str(args_str)?;
        let kwargs_str = ctx.get("kwargs").ok_or(anyhow!("Keyword args missing"))?;
        let kwargs: HashMap<String, serde_json::Value> = serde_json::from_str(kwargs_str)?;
        
        Ok(Self { definition, args, kwargs })
    }
}
