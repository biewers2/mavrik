use anyhow::Context;
use log::{debug, trace};
use crate::rb::{mavrik_error, module_mavrik, MRHash};
use crate::runtime::async_runtime;
use magnus::{function, method, Module, Object, RHash, Ruby};
use crate::tcp::{MavrikTcpClient, TcpClientOptions};
use crate::without_gvl;

#[derive(Debug)]
#[magnus::wrap(class = "Mavrik::Connection", free_immediately, size)]
pub struct RbClient(MavrikTcpClient);

impl RbClient {
    pub fn new(config: RHash) -> Result<Self, magnus::Error> {
        let config = MRHash(config);
        debug!(config:?; "Initializing client with config");
        
        let host = config.fetch_sym_or("host", "127.0.0.1".to_owned())?;
        let port = config.fetch_sym_or("port", 3001)?;
        let options = TcpClientOptions { host, port };

        let client = async_runtime()
            .block_on(async move { MavrikTcpClient::new(options).await })
            .map_err(mavrik_error)?;

        Ok(Self(client))
    }
    
    pub fn send_message(&self, message: RHash) -> Result<magnus::Value, magnus::Error> {
        without_gvl!({ self.send(message).map_err(mavrik_error) })
    }

    #[inline]
    fn send(&self, message: RHash) -> Result<magnus::Value, anyhow::Error> {
        async_runtime().block_on(async move {
            let request = message.try_into().context("converting hash to request failed")?;
            self.0.send(&request).await.context("sending request to server failed")?;
            let response = self.0.recv().await.context("receiving response from server failed")?;

            trace!(response:?; "Received response over TCP");
            let value = response.into();
            Ok(value)
        })
    }
}

pub fn define_client(ruby: &Ruby) -> Result<(), magnus::Error> {
    let client = module_mavrik().define_class("Connection", ruby.class_object())?;
    client.define_singleton_method("new", function!(RbClient::new, 1))?;
    client.define_method("send_message", method!(RbClient::send_message, 1))?;
    Ok(())
}
