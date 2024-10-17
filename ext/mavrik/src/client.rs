use crate::tcp_listener::SerialEvent;
use std::net::TcpStream;

pub struct ClientOptions {
    pub host: String,
    pub port: u16
}

#[derive(Debug)]
pub struct Client {
    stream: TcpStream
}

impl Client {
    pub fn new(options: ClientOptions) -> Self {
        let address = format!("{}:{}", options.host, options.port);
        let stream = TcpStream::connect(address).expect("failed to connect to TCP");

        Self { stream }
    }

    pub fn submit<'de, E: SerialEvent<'de>>(&mut self, task: &E) -> Result<(), anyhow::Error> {
        serde_json::to_writer(&mut self.stream, task)?;
        Ok(())
    }
}
