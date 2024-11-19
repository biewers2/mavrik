use crate::mavrik::MavrikOptions;
use crate::messaging::TaskId;
use crate::service::{start_service, MavrikService, ServiceChannel};
use crate::store::{PullStore, PushStore};
use crate::tcp::TcpClientHandler;
use anyhow::Context;
use libc::{getppid, kill, SIGUSR1};
use log::{info, warn};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinSet;

/// Mavrik's TCP listener struct, used to accept and manage asynchronous connections from clients and send received
/// messaging to the event loop.
pub struct MavrikTcpListener<Store> {
    inner: TcpListener,
    store: Store,
    handlers: JoinSet<Result<(), anyhow::Error>>,
    handler_chans: Vec<ServiceChannel<()>>
}

impl<Store> MavrikTcpListener<Store> {
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
    pub async fn new(options: &MavrikOptions, store: Store) -> Result<Self, anyhow::Error> {
        let host = options.get("host", "127.0.0.1".to_string())?;
        let port = options.get("port", 3001)?;
        let signal_parent_ready = options.get("signal_parent_ready", false)?;
        
        let inner = TcpListener::bind(format!("{host}:{port}")).await?;
        let handlers = JoinSet::new();
        let handler_chans = Vec::new();

        info!(host, port; "Accepting TCP connections");

        if signal_parent_ready {
            match unsafe { kill(getppid(), SIGUSR1) } {
                0 => info!("Successfully signalled ready to parent process"),
                _ => warn!("Failed to send ready signal to parent process")
            }
        }

        Ok(Self { inner, store, handlers, handler_chans })
    }
}

impl<Store> MavrikService for MavrikTcpListener<Store>
where
    Store: PushStore<Id = TaskId, Error = anyhow::Error>
        + PullStore<Id = TaskId, Error = anyhow::Error>
        + Clone + Send + Sync + 'static,
{
    type TaskOutput = Result<(TcpStream, SocketAddr), anyhow::Error>;
    type Message = ();

    // Accept TCP connections from client
    async fn poll_task(&mut self) -> Self::TaskOutput {
        self.inner.accept().await.context("failed to accept TCP connections")
    }

    // Handle TCP connections from client by spawning a new service task.
    async fn on_task_ready(&mut self, conn: Self::TaskOutput) -> Result<(), anyhow::Error> {
        let (stream, addr) = conn?;
        info!(addr:?; "Accepted connection");
        
        let handler = TcpClientHandler::new(stream, self.store.clone());
        let (handler, chan) = start_service("TCP-handler", handler);
        
        self.handlers.spawn(handler);
        self.handler_chans.push(chan);
        Ok(())
    }

    // Tell client handlers to terminate upon termination of this service.
    async fn on_terminate(&mut self) -> Result<(), anyhow::Error> {
        // Don't drop channel until all handlers are joined.
        // This avoids the TX being dropped while RX is still receiving resulting in an unexpected error
        // during termination.
        let mut term_chans = vec![];
        while let Some(mut chan) = self.handler_chans.pop() {
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
