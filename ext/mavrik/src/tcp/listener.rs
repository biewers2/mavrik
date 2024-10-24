use crate::events::{MavrikEvent, TcpEvent};
use crate::tcp::{TcpClientHandler, TcpClientHandlerParams};
use crate::service::{start_service, Service, ServiceChannel};
use libc::{getppid, kill, SIGUSR1};
use log::{info, warn};
use std::net::SocketAddr;
use anyhow::Context;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

/// Options for configuring the TCP listener.
pub struct TcpListenerOptions {
    /// The host to bind to.
    pub host: String,

    /// The port to bind to.
    pub port: u16,

    /// Whether to send SIGUSR1 to the parent process, indicating the TCP listener is ready to accept connections.
    pub signal_parent_ready: bool
}

/// Parameters for constructing the TCP listener.
pub struct TcpListenerParams {
    /// Where to send Mavrik events.
    pub event_tx: mpsc::Sender<MavrikEvent>,
}

/// Mavrik's TCP listener struct, used to accept and manage asynchronous connections from clients and send received
/// events to the event loop.
pub struct MavrikTcpListener {
    /// The inner tokio TCP listener.
    inner: TcpListener,

    /// Where to send Mavrik events.
    event_tx: mpsc::Sender<MavrikEvent>,

    /// Set of asynchronous tasks handling client streams.
    handlers: JoinSet<Result<(), anyhow::Error>>,

    /// Oneshot senders for terminating the client stream handling tasks.
    /// Allows this listener to safely ensure the tasks in the internal join set return successfully before exiting to the parent
    /// task.
    handler_chans: Vec<ServiceChannel<()>>
}

impl MavrikTcpListener {
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
    pub async fn new(options: TcpListenerOptions, params: TcpListenerParams) -> Result<Self, anyhow::Error> {
        let TcpListenerOptions {
            host,
            port,
            signal_parent_ready
        } = options;
        let TcpListenerParams { event_tx } = params;
        
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

        Ok(Self { inner, event_tx, handlers, handler_chans })
    }
}

impl Service for MavrikTcpListener {
    type TaskOutput = Result<(TcpStream, SocketAddr), anyhow::Error>;
    type Message = TcpEvent;

    // Accept TCP connections from client
    async fn poll_task(&mut self) -> Self::TaskOutput {
        self.inner.accept().await.context("failed to accept TCP connections")
    }

    // Handle TCP connections from client by spawning a new service task.
    async fn on_task_ready(&mut self, conn: Self::TaskOutput) -> Result<(), anyhow::Error> {
        let (stream, addr) = conn?;
        info!(addr:?; "Accepted connection");
        
        let event_tx = self.event_tx.clone();
        let handler = TcpClientHandler::new(stream, TcpClientHandlerParams { event_tx });
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
