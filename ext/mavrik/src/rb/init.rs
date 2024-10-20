use crate::rb::client::RbClient;
use crate::rb::{mavrik_error, module_mavrik};
use crate::runtime::async_runtime;
use log::debug;
use magnus::{function, Object, RHash, Ruby};
use crate::fetch;
use crate::io::{Client, ClientOptions};

pub(crate) fn define_init(_ruby: &Ruby) -> Result<(), magnus::Error> {
    module_mavrik().define_singleton_method("init", function!(init, 1))?;
    Ok(())
}

fn init(config: RHash) -> Result<RbClient, magnus::Error> {
    debug!("Initializing client with config {config:?}");

    let host = fetch!(config, :"host", "127.0.0.1".to_owned())?;
    let port = fetch!(config, :"port", 3001)?;
    let options = ClientOptions { host, port };
    
    let client = async_runtime()
        .block_on(async move { Client::new(options).await })
        .map_err(mavrik_error)?;
    
    Ok(RbClient::new(client))
}
