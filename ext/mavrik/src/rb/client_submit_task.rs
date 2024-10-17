use crate::rb::class_mavrik_client;
use crate::rb::client::RbClient;
use crate::tcp_listener::SerialEvent;
use magnus::{function, Object, Ruby};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[magnus::wrap(class = "Mavrik::SubmittedTask", free_immediately, size)]
pub struct SubmittedTask {
    pub queue: String,
    pub definition: String, // repr class path
    pub input_args: String, // repr JSON array
    pub input_kwargs: String, // repr JSON object
}

impl SerialEvent<'_> for SubmittedTask {}

pub(crate) fn define_client_submit_task(ruby: &Ruby) -> Result<(), magnus::Error> {
    let client = class_mavrik_client(ruby)?;
    client.define_singleton_method("submit_task", function!(client_submit_task, 1))?;
    Ok(())
}

fn client_submit_task(client: &RbClient) -> Result<(), magnus::Error> {
    Ok(())
}
