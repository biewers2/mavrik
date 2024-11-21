use anyhow::Context;
use log::debug;
use crate::rb::{mavrik_error, module_mavrik, MRHash};
use crate::runtime::async_runtime;
use magnus::{function, method, Module, Object, RHash, Ruby};
use crate::messaging::{MavrikRequest, MavrikResponse};
use crate::tcp::{MavrikTcpClient, TcpClientOptions};
use crate::without_gvl;

#[derive(Debug)]
#[magnus::wrap(class = "Mavrik::Connection", free_immediately, size)]
pub struct RbConnection {
    tcp_client: MavrikTcpClient
}

impl RbConnection {
    pub fn new(config: RHash) -> Result<Self, magnus::Error> {
        let config = MRHash(config);
        debug!(config:?; "Initializing client with config");

        let host = config.fetch_sym_or("host", "127.0.0.1".to_owned())?;
        let port = config.fetch_sym_or("port", 3001)?;
        let options = TcpClientOptions { host, port };

        let tcp_client = async_runtime()
            .block_on(async move { MavrikTcpClient::new(options).await })
            .map_err(mavrik_error)?;

        Ok(Self { tcp_client })
    }
    
    pub fn request(&self, req: RHash) -> Result<magnus::Value, magnus::Error> {
        let req = req.try_into()?;
        let res = without_gvl!({ self.send(&req).map_err(mavrik_error) })?;
        Ok(res.into())
    }

    #[inline]
    fn send(&self, req: &MavrikRequest) -> Result<MavrikResponse, anyhow::Error> {
        async_runtime().block_on(async move {
            self.tcp_client.send(req).await.context("sending request to server failed")?;
            let res = self.tcp_client.recv().await.context("receiving response from server failed")?;
            Ok(res)
        })
    }
}

pub fn define_client(ruby: &Ruby) -> Result<(), magnus::Error> {
    let client = module_mavrik().define_class("Connection", ruby.class_object())?;
    client.define_singleton_method("new", function!(RbConnection::new, 1))?;
    client.define_method("request", method!(RbConnection::request, 1))?;
    Ok(())
}
