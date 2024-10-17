use log::debug;
use crate::rb::module_mavrik;
use magnus::{function, Object, RHash, Ruby, TryConvert};
use crate::client::{Client, ClientOptions};
use crate::rb::client::RbClient;

pub(crate) fn define_init(ruby: &Ruby) -> Result<(), magnus::Error> {
    let mavrik = module_mavrik(ruby);
    mavrik.define_singleton_method("init", function!(init, 1))?;
    Ok(())
}

fn init(config: RHash) -> Result<RbClient, magnus::Error> {
    debug!("Initializing client with config {config:?}");

    let host = fetch(&config, "host", "127.0.0.1".to_owned())?;
    let port = fetch(&config, "port", 3009)?;
    let options = ClientOptions { host, port };
    
    Ok(RbClient(Client::new(options)))
}

fn fetch<T: TryConvert>(hash: &RHash, key: &str, default: T) -> Result<T, magnus::Error> {
    let value = hash
        .get(key)
        .map(|v| T::try_convert(v))
        .transpose()?
        .unwrap_or(default);

    Ok(value)
}
