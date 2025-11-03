use crate::messaging::task_id::TaskId;
use crate::messaging::NewTask;
use crate::store::StoreState;
use serde::{Deserialize, Serialize};

/// A request made from a TCP client to the TCP listener service ("TCP").
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MavrikRequest {
    /// A new task being submitted.
    NewTask(NewTask),

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

// impl TryFrom<RHash> for MavrikRequest {
//     type Error = magnus::Error;
//
//     fn try_from(h: RHash) -> Result<Self, Self::Error> {
//         let h = MRHash(h);
//
//         let variant = h.try_fetch_sym::<String>("type")?;
//         match variant.as_str() {
//             "new_task" => {
//                 let new_task = NewTask {
//                     queue: h.try_fetch_sym("queue")?,
//                     definition: h.try_fetch_sym("definition")?,
//                     args: h.try_fetch_sym("args")?,
//                     kwargs: h.try_fetch_sym("kwargs")?,
//                 };
//
//                 Ok(MavrikRequest::NewTask(new_task))
//             },
//
//             "get_store_state" => Ok(MavrikRequest::GetStoreState),
//
//             _ => Err(mavrik_error(anyhow!("unsupported request type: {}", variant))),
//         }
//     }
// }
//
// macro_rules! hset {
//     ($hash:ident, :$key:expr, $value:expr) => {
//         $hash.set_sym($key, $value).expect(&format!("failed to set '{}'", $key))
//     };
// }
//
// impl IntoValue for MavrikResponse {
//     fn into_value_with(self, ruby: &Ruby) -> magnus::Value {
//         match self {
//             MavrikResponse::NewTaskId(task_id) => {
//                 task_id.to_string().into_value_with(ruby)
//             },
//
//             MavrikResponse::StoreState(state) => {
//                 let state_h = MRHash::new();
//
//                 let task_hs = ruby.ary_new();
//                 for task in state.tasks {
//                     let h = MRHash::new();
//                     hset!(h, :"id", format!("{}", task.id));
//                     hset!(h, :"status", serde_json::to_string(&task.status).unwrap());
//                     hset!(h, :"definition", task.definition);
//                     hset!(h, :"args", task.args);
//                     hset!(h, :"kwargs", task.kwargs);
//                     task_hs.push(h.into_value_with(ruby)).expect("failed to push task hash");
//                 }
//                 hset!(state_h, :"tasks", task_hs);
//
//                 state_h.into_value_with(ruby)
//             }
//         }
//     }
// }
