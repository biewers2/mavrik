use crate::events::{MavrikRequest, MavrikResponse};
use crate::io::{read_deserialized_async, write_serialized_async};
use async_std::net::TcpStream;
use async_std::sync::Mutex;
use std::ops::DerefMut;

pub struct ClientOptions {
    pub host: String,
    pub port: u16
}

#[derive(Debug)]
pub struct Client {
    stream: Mutex<TcpStream>
}

impl Client {
    pub async fn new(options: ClientOptions) -> Result<Self, anyhow::Error> {
        let address = format!("{}:{}", options.host, options.port);
        let stream = TcpStream::connect(address).await?;
        let stream = Mutex::new(stream);

        Ok(Self { stream })
    }

    pub async fn send(&self, request: &MavrikRequest) -> Result<(), anyhow::Error> {
        let mut stream = self.stream.lock().await;
        write_serialized_async(stream.deref_mut(), &request).await?;
        Ok(())
    }

    pub async fn recv(&self) -> Result<MavrikResponse, anyhow::Error> {
        let mut stream = self.stream.lock().await;
        let response = read_deserialized_async(stream.deref_mut()).await?;
        Ok(response)
    }
}
