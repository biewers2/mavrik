use anyhow::Context;
use log::trace;
use crate::rb::{mavrik_error, module_mavrik};
use crate::runtime::async_runtime;
use magnus::{method, Module, Ruby};
use crate::tcp::MavrikTcpClient;
use crate::without_gvl;

#[derive(Debug)]
#[magnus::wrap(class = "Mavrik::Connection", free_immediately, size)]
pub struct RbClient(MavrikTcpClient);

impl RbClient {
    pub fn new(inner: MavrikTcpClient) -> Self {
        Self(inner)
    }
    
    pub fn send_message(&self, message: String) -> Result<String, magnus::Error> {
        without_gvl!({ self.send(&message).map_err(mavrik_error) })
    }

    #[inline]
    fn send(&self, message: &str) -> Result<String, anyhow::Error> {
        async_runtime().block_on(async move {
            let request = serde_json::from_str(message).context("deserializing request from message")?;
            trace!(request:?; "Sending request over TCP");

            self.0.send(&request).await.context("sending request to server")?;
            let response = self.0.recv().await.context("receiving response from server")?;

            trace!(response:?; "Received response over TCP");
            let value = serde_json::to_string(&response).context("serialize response to message")?;
            Ok(value)    
        })
    }
}

pub fn define_client(ruby: &Ruby) -> Result<(), magnus::Error> {
    let client = module_mavrik().define_class("Client", ruby.class_object())?;
    client.define_method("send_message", method!(RbClient::send_message, 1))?;
    Ok(())
}
