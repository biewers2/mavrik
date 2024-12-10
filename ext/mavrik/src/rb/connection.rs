use crate::messaging::{MavrikRequest, MavrikResponse};
use crate::rb::util::{mavrik_error, module_mavrik, MRHash};
use crate::runtime::async_runtime;
use crate::tcp::{MavrikTcpClient, TcpClientOptions};
use crate::without_gvl;
use anyhow::Context;
use log::debug;
use magnus::{function, method, Module, Object, RHash, Ruby};

pub fn define_connection(ruby: &Ruby) -> Result<(), magnus::Error> {
    let conn = module_mavrik().define_class("Connection", ruby.class_object())?;
    conn.define_singleton_method("new", function!(RbConnection::new, 1))?;
    conn.define_method("request", method!(RbConnection::request, 1))?;
    Ok(())
}

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

#[cfg(test)]
pub mod tests {
    use std::future::Future;
    use crate::rb::connection::{define_connection, RbConnection};
    use crate::rb::util::{mavrik_error, module_mavrik, MRHash};
    use magnus::{Class, Module, RClass, RHash, Ruby};
    use std::net::SocketAddr;
    use tokio::net::{TcpListener, TcpStream};
    use std::thread;
    use std::thread::JoinHandle;
    use magnus::value::ReprValue;
    use crate::io::{read_object, write_object};
    use crate::messaging::{MavrikRequest, MavrikResponse};
    use crate::runtime::async_runtime;
    use crate::store::StoreState;

    pub fn define_connection_defines_ruby_class_and_methods(r: &Ruby) -> Result<(), magnus::Error> {
        define_connection(r)?;

        let class_conn: RClass = module_mavrik().const_get("Connection")?;
        assert_eq!(unsafe { class_conn.name() }, "Mavrik::Connection");
        assert!(class_conn.respond_to("new", false)?);

        let conn = class_conn.new_instance(())?;
        assert!(conn.respond_to("request", false)?);

        Ok(())
    }

    pub fn new_connection_connects_to_server(_r: &Ruby) -> Result<(), magnus::Error> {
        let host = "127.0.0.1";
        let port = 2999;
        let config = MRHash::new();
        config.set_sym("host", host)?;
        config.set_sym("port", port)?;
        let handle = set_up_listener(host, port, |(_, addr)| async move { addr })
            .map_err(mavrik_error)?;
        
        let _ = RbConnection::new(config.into())?;
        
        let result = handle.join().unwrap();
        assert_eq!(result.ip().to_string(), host);
        
        Ok(())
    }
    
    pub fn new_connection_fails_to_connect_to_server(_r: &Ruby) -> Result<(), magnus::Error> {
        let host = "127.0.0.1";
        let port = 2998;
        let config = new_config(host, port)?;

        let result = RbConnection::new(config.into());
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Connection refused"));
        Ok(())
    }
    
    pub fn new_connection_requests_data_from_server(_r: &Ruby) -> Result<(), magnus::Error> {
        let host = "127.0.0.1";
        let port = 2997;
        let config = new_config(host, port)?;
        let handle = set_up_listener(host, port, |(mut stream, _)| async move {
            let req: MavrikRequest = read_object(&mut stream).await.unwrap();
            assert_eq!(req, MavrikRequest::GetStoreState);
            
            let res = MavrikResponse::StoreState(StoreState { tasks: vec![] });
            write_object(&mut stream, res).await.unwrap();
        }).map_err(mavrik_error)?;
        
        let req = MRHash::new();
        req.set_sym("type", "get_store_state")?;
        let conn = RbConnection::new(config.into())?;
        let res = conn.request(req.into())?;
        
        handle.join().unwrap();
        assert!(!res.is_nil());
        Ok(())
    }
    
    fn new_config(host: &str, port: u16) -> Result<RHash, magnus::Error> {
        let config = MRHash::new();
        config.set_sym("host", host)?;
        config.set_sym("port", port)?;
        Ok(config.into())
    }

    fn set_up_listener<T, F, Fut>(host: impl Into<String>, port: u16, block: F) -> Result<JoinHandle<T>, anyhow::Error>
    where
        T: Send + 'static,
        F: FnOnce((TcpStream, SocketAddr)) -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        let rt = async_runtime();
        
        let listener = rt.block_on(TcpListener::bind(format!("{}:{}", host.into(), port)))?;
        let handle = thread::spawn(move || rt.block_on(async move {
            let accepted = listener.accept().await.unwrap();
            block(accepted).await
        }));
        
        Ok(handle)
    }
}
