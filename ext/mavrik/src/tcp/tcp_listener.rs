use crate::events::{MavrikEvent, TcpEvent};
use crate::tcp::handle_tcp_stream::{handle_tcp_stream, HandleTcpStreamParams};
use crate::service::Service;
use libc::{getppid, kill, SIGUSR1};
use log::{info, warn};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
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
    conns: JoinSet<Result<(), anyhow::Error>>,

    /// Oneshot senders for terminating the client stream handling tasks.
    /// Allows this listener to safely ensure the tasks in the internal join set return successfully before exiting to the parent
    /// task.
    conn_term_txs: Vec<oneshot::Sender<()>>
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
        let conns = JoinSet::new();
        let conn_term_txs = Vec::new();

        info!("Accepting TCP connections on {host}:{port}");

        if signal_parent_ready {
            match unsafe { kill(getppid(), SIGUSR1) } {
                0 => info!("Successfully signalled ready to parent process"),
                _ => warn!("Failed to send ready signal to parent process")
            }
        }

        Ok(Self { inner, event_tx, conns, conn_term_txs })
    }
}

impl Service for MavrikTcpListener {
    type TaskOutput = Result<(TcpStream, SocketAddr), anyhow::Error>;
    type Message = TcpEvent;

    async fn call_task(&mut self) -> Self::TaskOutput {
        let conn = self.inner.accept().await?;
        Ok(conn)
    }

    async fn on_task_ready(&mut self, conn: Self::TaskOutput) -> Result<(), anyhow::Error> {
        let (conn_term_tx, conn_term_rx) = oneshot::channel();
        
        let (stream, addr) = conn?;
        info!("Accepted connection from {addr:?}");
        
        self.conns.spawn(handle_tcp_stream(stream, HandleTcpStreamParams {
            event_tx: self.event_tx.clone(),
            term_rx: conn_term_rx
        }));
        self.conn_term_txs.push(conn_term_tx);
        Ok(())
    }

    async fn on_message(&mut self, _message: Self::Message) -> Result<(), anyhow::Error> {
        Ok(())
    }

    async fn on_terminate(&mut self) -> Result<(), anyhow::Error> {
        while let Some(term_tx) = self.conn_term_txs.pop() {
            let _ = term_tx.send(());
        }

        while let Some(result) = self.conns.join_next().await {
            result??;
        }

        Ok(())
    }
}
