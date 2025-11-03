use crate::messaging::{MavrikRequest, MavrikResponse};
use crate::rb::util::{mavrik_error, module_mavrik};
use crate::runtime::async_runtime;
use crate::tcp::{MavrikTcpClient, TcpClientOptions};
use crate::{ruby_or_mavrik_error, without_gvl};
use anyhow::Context;
use log::debug;
use magnus::{function, method, Module, Object, Ruby};
use serde::{Deserialize, Serialize};
use serde_magnus::{deserialize, serialize};

pub fn define_connection(ruby: &Ruby) -> Result<(), magnus::Error> {
    let conn = module_mavrik().define_class("Connection", ruby.class_object())?;
    conn.define_singleton_method("new", function!(RbConnection::new, 1))?;
    conn.define_method("request", method!(RbConnection::request, 1))?;
    Ok(())
}

#[derive(Debug)]
#[magnus::wrap(class = "Mavrik::Connection", free_immediately, size)]
pub struct RbConnection {
    tcp_client: MavrikTcpClient,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbConnectionConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
}

impl RbConnection {
    pub fn new(config: magnus::Value) -> Result<Self, magnus::Error> {
        let ruby = ruby_or_mavrik_error!()?;
        let config: RbConnectionConfig = deserialize(&ruby, config)?;
        debug!(config:?; "Initializing client with config");

        let host = config.host.unwrap_or("127.0.0.1".to_owned());
        let port = config.port.unwrap_or(3001);
        let options = TcpClientOptions { host, port };

        let tcp_client = async_runtime()
            .block_on(async move { MavrikTcpClient::new(options).await })
            .map_err(mavrik_error)?;

        Ok(Self { tcp_client })
    }

    pub fn request(&self, req: magnus::Value) -> Result<magnus::Value, magnus::Error> {
        let ruby = ruby_or_mavrik_error!()?;
        let req = deserialize(&ruby, req)?;
        let res = without_gvl!({ self.send(&req).map_err(mavrik_error) })?;
        serialize(&ruby, &res)
    }

    #[inline]
    fn send(&self, req: &MavrikRequest) -> Result<MavrikResponse, anyhow::Error> {
        async_runtime().block_on(async move {
            self.tcp_client
                .send(req)
                .await
                .context("sending request to server failed")?;
            let res = self
                .tcp_client
                .recv()
                .await
                .context("receiving response from server failed")?;
            Ok(res)
        })
    }
}

#[cfg(test)]
pub mod tests {
    use crate::io::{read_object, write_object};
    use crate::messaging::{MavrikRequest, MavrikResponse};
    use crate::rb::connection::{define_connection, RbConnection, RbConnectionConfig};
    use crate::rb::util::{mavrik_error, module_mavrik};
    use crate::runtime::async_runtime;
    use crate::store::StoreState;
    use magnus::value::ReprValue;
    use magnus::{Class, Module, RClass, Ruby};
    use serde_magnus::serialize;
    use std::future::Future;
    use std::net::SocketAddr;
    use std::thread;
    use std::thread::JoinHandle;
    use tokio::net::{TcpListener, TcpStream};

    pub fn define_connection_defines_ruby_class_and_methods(r: &Ruby) -> Result<(), magnus::Error> {
        define_connection(r)?;

        let class_conn: RClass = module_mavrik().const_get("Connection")?;
        assert_eq!(unsafe { class_conn.name() }, "Mavrik::Connection");
        assert!(class_conn.respond_to("new", false)?);

        let conn = class_conn.new_instance(())?;
        assert!(conn.respond_to("request", false)?);

        Ok(())
    }

    pub fn new_connection_connects_to_server(ruby: &Ruby) -> Result<(), magnus::Error> {
        let host = "127.0.0.1";
        let port = 2999;
        let config = RbConnectionConfig {
            host: Some(String::from(host)),
            port: Some(port),
        };
        let handle =
            set_up_listener(host, port, |(_, addr)| async move { addr }).map_err(mavrik_error)?;

        let _ = RbConnection::new(serialize(ruby, &config)?)?;

        let result = handle.join().unwrap();
        assert_eq!(result.ip().to_string(), host);

        Ok(())
    }

    pub fn new_connection_fails_to_connect_to_server(ruby: &Ruby) -> Result<(), magnus::Error> {
        let host = "127.0.0.1";
        let port = 2998;
        let config = RbConnectionConfig {
            host: Some(String::from(host)),
            port: Some(port),
        };

        let result = RbConnection::new(serialize(ruby, &config)?);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Connection refused"));
        Ok(())
    }

    pub fn new_connection_requests_data_from_server(ruby: &Ruby) -> Result<(), magnus::Error> {
        let host = "127.0.0.1";
        let port = 2997;
        let config = RbConnectionConfig {
            host: Some(String::from(host)),
            port: Some(port),
        };
        let handle = set_up_listener(host, port, |(mut stream, _)| async move {
            let req: MavrikRequest = read_object(&mut stream).await.unwrap();
            assert_eq!(req, MavrikRequest::GetStoreState);

            let res = MavrikResponse::StoreState(StoreState { tasks: vec![] });
            write_object(&mut stream, res).await.unwrap();
        })
        .map_err(mavrik_error)?;

        let conn = RbConnection::new(serialize(ruby, &config)?)?;
        let res = conn.request(serialize(ruby, &MavrikRequest::GetStoreState)?)?;

        handle.join().unwrap();
        assert!(!res.is_nil());
        Ok(())
    }

    fn set_up_listener<T, F, Fut>(
        host: &str,
        port: u16,
        block: F,
    ) -> Result<JoinHandle<T>, anyhow::Error>
    where
        T: Send + 'static,
        F: FnOnce((TcpStream, SocketAddr)) -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        let rt = async_runtime();

        let listener = rt.block_on(TcpListener::bind(format!("{}:{}", host, port)))?;
        let handle = thread::spawn(move || {
            rt.block_on(async move {
                let accepted = listener.accept().await.unwrap();
                block(accepted).await
            })
        });

        Ok(handle)
    }
}
