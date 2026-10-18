use crate::messaging::TaskId;
use crate::server::TcpClientHandler;
use crate::service::{ServiceChannel, ServiceTask, Services};
use crate::store::{PullStoreOutput, PushStoreInput, QueryStore};
use anyhow::Context;
use libc::{getppid, kill, SIGUSR1};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerOptions {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub signal_parent_ready: Option<bool>,
}

impl ServerOptions {
    pub fn get_host(&self) -> &str {
        self.host.as_deref().unwrap_or("127.0.0.1")
    }

    pub fn get_port(&self) -> u16 {
        self.port.unwrap_or(3001)
    }

    pub fn should_signal_parent_ready(&self) -> bool {
        self.signal_parent_ready.unwrap_or(false)
    }
}

/// Mavrik's TCP listener struct, used to accept and manage asynchronous connections from clients and send received
/// messaging to the event loop.
pub struct Server<Store> {
    inner: TcpListener,
    store: Store,
    handlers: JoinSet<Result<(), anyhow::Error>>,
    handler_channels: Vec<ServiceChannel>,
}

impl<Store> Server<Store> {
    /// Bind this listener to an address with a Mavrik event sender.
    ///
    /// # Arguments
    ///
    /// `options` - Options used for configuration.
    /// `params` - Parameters used for initialization.
    ///
    /// # Returns
    ///
    /// A result containing a new Mavrik TCP listener on OK, otherwise any error that occurred.
    ///
    pub async fn new(options: &ServerOptions, store: Store) -> Result<Self, anyhow::Error> {
        let host = options.get_host();
        let port = options.get_port();
        let signal_parent_ready = options.should_signal_parent_ready();

        let inner = TcpListener::bind(format!("{host}:{port}")).await?;
        let handlers = JoinSet::new();
        let handler_channels = Vec::new();

        info!(host, port; "Accepting TCP connections");

        if signal_parent_ready {
            match unsafe { kill(getppid(), SIGUSR1) } {
                0 => info!("Successfully signalled ready to parent process"),
                code => {
                    warn!("Failed to send ready signal to parent process: kill exit code {code}")
                }
            }
        }

        Ok(Self {
            inner,
            store,
            handlers,
            handler_channels,
        })
    }
}

impl<Store> ServiceTask for Server<Store>
where
    Store: PushStoreInput<Id = TaskId, Error = anyhow::Error>
        + PullStoreOutput<Id = TaskId, Error = anyhow::Error>
        + QueryStore<Error = anyhow::Error>
        + Clone
        + Send
        + Sync
        + 'static,
{
    type ReadyTask = Result<(TcpStream, SocketAddr), anyhow::Error>;

    // Accept TCP connections from client
    async fn poll_task(&mut self) -> Self::ReadyTask {
        self.inner
            .accept()
            .await
            .context("failed to accept TCP connections")
    }

    // Handle TCP connections from client by spawning a new service task.
    async fn on_task_ready(&mut self, conn: Self::ReadyTask) -> Result<(), anyhow::Error> {
        let (stream, addr) = conn?;
        info!(addr:?; "Accepted connection");

        let service = Services::start(
            "TCP-handler",
            TcpClientHandler::new(stream, self.store.clone()),
        );

        self.handlers.spawn(service.task);
        self.handler_channels.push(service.channel);
        Ok(())
    }

    // Tell client handlers to terminate upon termination of this service.
    async fn on_terminate(&mut self) -> Result<(), anyhow::Error> {
        // Don't drop channel until all handlers are joined.
        // This avoids the TX being dropped while RX is still receiving resulting in an unexpected error
        // during termination.
        let mut term_chans = vec![];
        while let Some(mut chan) = self.handler_channels.pop() {
            chan.terminate();
            term_chans.push(chan);
        }

        while let Some(result) = self.handlers.join_next().await {
            result
                .context("failed to join client handler")?
                .context("client handler failed during execution")?;
        }

        Ok(())
    }
}
