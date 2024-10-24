use std::ops::DerefMut;
use anyhow::Context;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use crate::events::{MavrikRequest, MavrikResponse};
use crate::tcp::{read_deserialized, write_serialized};

/// Options for creating a TCP client.
pub struct TcpClientOptions {
    /// The host to connect to.
    pub host: String,
    
    /// The port to connect on.
    pub port: u16
}

/// The TCP client used to communicate w/ the Mavrik server.
#[derive(Debug)]
pub struct MavrikTcpClient {
    /// The TCP stream once connected.
    stream: Mutex<TcpStream>
}

impl MavrikTcpClient {
    /// Connect to the Mavrik server.
    /// 
    /// # Arguments
    /// 
    /// `options` - The options to use when connecting to the server.
    /// 
    /// # Returns
    /// 
    /// A result containing the new client on success.
    /// 
    pub async fn new(options: TcpClientOptions) -> Result<Self, anyhow::Error> {
        let address = format!("{}:{}", options.host, options.port);
        let stream = TcpStream::connect(address).await.context("failed to connect via TCP")?;
        let stream = Mutex::new(stream);

        Ok(Self { stream })
    }

    /// Send a request to the server.
    /// 
    /// # Arguments
    /// 
    /// `request` - The request to send
    ///
    pub async fn send(&self, request: &MavrikRequest) -> Result<(), anyhow::Error> {
        let mut stream = self.stream.lock().await;
        write_serialized(stream.deref_mut(), &request).await.context("sending Mavrik request over TCP")?;
        Ok(())
    }

    /// Receive a response from the server.
    /// 
    /// # Returns
    /// 
    /// The response from the server.
    /// 
    pub async fn recv(&self) -> Result<MavrikResponse, anyhow::Error> {
        let mut stream = self.stream.lock().await;
        let response = read_deserialized(stream.deref_mut()).await.context("receiving Mavrik response over TCP")?;
        Ok(response)
    }
}
