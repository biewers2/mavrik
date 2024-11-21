use crate::messaging::task_id::TaskId;
use crate::messaging::NewTask;
use crate::rb::{mavrik_error, MRHash};
use crate::store::StoreState;
use anyhow::{anyhow, Context};
use magnus::{IntoValue, RArray, RHash};
use serde::{Deserialize, Serialize};

/// A request made from a TCP client to the TCP listener service ("TCP").
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MavrikRequest {
    /// A new task being submitted.
    NewTask(NewTask),
    
    /// Get the state of the storage container.
    GetStoreState
}

/// A response given to a TCP client from the TCP listener service ("TCP").
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum MavrikResponse {
    /// The response for submitting a new task.
    /// Contains the created ID of the task submitted.
    NewTaskId(TaskId),

    /// The state of the storage container.
    StoreState(StoreState),
}

macro_rules! fetch_required {
    ($hash:ident, $key:expr) => {
        $hash.fetch_sym($key).and_then(|v| v.ok_or(crate::rb::mavrik_error(anyhow!("{} missing", $key))))
    };
}

impl TryFrom<RHash> for MavrikRequest {
    type Error = magnus::Error;

    fn try_from(h: RHash) -> Result<Self, Self::Error> {
        let h = MRHash(h);
        
        let variant: String = fetch_required!(h, "type")?;
        match variant.as_str() {
            "new_task" => {
                let new_task = NewTask {
                    queue: fetch_required!(h, "queue")?,
                    definition: fetch_required!(h, "definition")?,
                    args: fetch_required!(h, "args")?,
                    kwargs: fetch_required!(h, "kwargs")?,
                };
                
                Ok(MavrikRequest::NewTask(new_task))
            },
            
            "get_store_state" => Ok(MavrikRequest::GetStoreState),
            
            _ => Err(mavrik_error(anyhow!("unsupported request type: {}", variant))),
        }
    }
}

macro_rules! hset {
    ($hash:ident, :$key:expr, $value:expr) => {
        $hash.set_sym($key, $value).expect(&format!("failed to set '{}'", $key))
    };
}

impl From<MavrikResponse> for magnus::Value {
    fn from(response: MavrikResponse) -> Self {
        match response {
            MavrikResponse::NewTaskId(task_id) => {
                format!("{task_id}").into_value()
            },
            
            MavrikResponse::StoreState(state) => {
                let state_h = MRHash::new();
                
                let task_hs = RArray::new();
                for task in state.tasks {
                    let h = MRHash::new();
                    hset!(h, :"id", format!("{}", task.id));
                    hset!(h, :"status", serde_json::to_string(&task.status).unwrap());
                    hset!(h, :"definition", task.definition);
                    hset!(h, :"args", task.args);
                    hset!(h, :"kwargs", task.kwargs);
                    task_hs.push(h.into_value()).expect("failed to push task hash");
                }
                hset!(state_h, :"tasks", task_hs);
                
                state_h.into_value()
            }
        }
    }
}
