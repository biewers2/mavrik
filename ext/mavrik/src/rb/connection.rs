use anyhow::Context;
use log::debug;
use crate::rb::util::{mavrik_error, module_mavrik, MRHash};
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

#[cfg(test)]
pub mod tests {
    use std::io::{Read, Write};
    use magnus::Ruby;
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::thread;
    use std::thread::JoinHandle;
    use crate::messaging::{MavrikResponse, TaskId};
    use crate::rb::connection::RbConnection;
    use crate::rb::util::{mavrik_error, MRHash};
    use crate::runtime::async_runtime;

    pub fn test_connection_new_connects_to_server(_r: &Ruby) -> Result<(), magnus::Error> {
        let host = "127.0.0.1";
        let port = 2999;
        let config = MRHash::new();
        config.set_sym("host", host)?;
        config.set_sym("port", port)?;
        let handle = set_up_listener(host, port, |(stream, addr)| (stream, addr));
        
        let _ = RbConnection::new(config.into())?;
        
        let result = handle.join().map_err(mavrik_error)?;
        assert!(result.is_ok());
        
        Ok(())
    }
    
    pub fn test_connection_new_fails_to_connect_to_server(_r: &Ruby) -> Result<(), magnus::Error> {
        let host = "127.0.0.1";
        let port = 2998;
        let config = MRHash::new();
        config.set_sym("host", host)?;
        config.set_sym("port", port)?;

        let result = RbConnection::new(config.into());
        
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        println!("{}", error_message);
        assert!(error_message.contains("Connection refused"));
        Ok(())
    }
    
    pub fn test_connection_requests_data_from_server(_r: &Ruby) -> Result<(), magnus::Error> {
        let host = "127.0.0.1";
        let port = 2997;
        let config = MRHash::new();
        config.set_sym("host", host)?;
        config.set_sym("port", port)?;

        // Set up mock server that returns a task ID
        let handle = set_up_listener(host, port, |(mut stream, _addr)| {
            async_runtime().block_on(async {
                use crate::io::{read_object, write_object};
                use crate::messaging::MavrikRequest;
                
                // Read the request
                let request: MavrikRequest = read_object(&mut stream).await.unwrap();
                
                // Send back a task ID response 
                let response = MavrikResponse::StoreState(vec![]);  // Using StoreState since NewTaskId is private
                write_object(&mut stream, &response).await.unwrap();
            });
            
            stream
        });

        // Create connection and make request
        let conn = RbConnection::new(config.into())?;
        let request = MRHash::new();
        request.set_sym("type", "new_task")?;
        let result = conn.request(request.into())?;

        // Verify response
        let result_hash: RHash = result.try_convert()?;
        let mrh = MRHash(result_hash);
        let tasks = mrh.try_fetch_sym::<Vec<magnus::Value>>("tasks")?;
        assert_eq!(tasks.len(), 0);
        
        handle.join().map_err(mavrik_error)?;
        Ok(())
    }

    fn set_up_listener<T, F>(host: impl Into<String>, port: u16, block: F) -> JoinHandle<Result<T, anyhow::Error>>
    where
        T: Send + 'static,
        F: FnOnce((TcpStream, SocketAddr)) -> T + Send + 'static,
    {
        let host = host.into();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let handle = thread::spawn(move || {
            let listener = TcpListener::bind(format!("{}:{}", host, port))?;
            ready_tx.send(()).unwrap();
            let accepted = listener.accept()?;
            Ok(block(accepted))
        });
        ready_rx.recv().unwrap();
        
        handle
    }
}
