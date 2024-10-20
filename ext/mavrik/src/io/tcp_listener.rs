use std::future::IntoFuture;
use std::net::SocketAddr;
use crate::events::MavrikEvent;
use libc::{getppid, kill, SIGUSR1};
use log::{debug, info, warn};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::select;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use crate::io::handle_tcp_stream::{handle_tcp_stream, HandleTcpStreamParams};

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
    
    /// Where to listen for termination signals from the parent async task.
    pub term_rx: oneshot::Receiver<()>
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
    /// `addrs` - Socket address to bind to.
    /// `event_tx` - Where to send Mavrik events.
    /// 
    /// # Returns
    /// 
    /// A result containing a new Mavrik TCP listener on OK, otherwise any error that occurred.
    /// 
    pub async fn bind<A: ToSocketAddrs>(addrs: A, event_tx: mpsc::Sender<MavrikEvent>) -> Result<Self, anyhow::Error> {
        let inner = TcpListener::bind(addrs).await?;
        let conns = JoinSet::new();
        let conn_term_txs = Vec::new();
        
        Ok(Self { inner, event_tx, conns, conn_term_txs })
    }
    
    /// Listen for the next incoming connection.
    /// 
    /// # Returns
    /// 
    /// A result containing the TCP stream of the new connection (0), and the socket address of the client (1).
    /// 
    pub async fn listen(&self) -> Result<(TcpStream, SocketAddr), anyhow::Error> {
        let conn = self.inner.accept().await?;
        Ok(conn)
    }
    
    /// Accept and handle a connection from the client.
    /// 
    /// The connection is accepted by passing the TCP stream to the client handling logic in `handle_tcp_stream`.
    /// The oneshot termination sender is then pushed to this listener's vec of senders.
    /// 
    /// `Arguments`
    /// 
    /// `(stream, addr)` - The TCP stream/client address that was accepted by the inner TCP listener.
    /// 
    /// `Returns`
    /// 
    /// The socket address of the client.
    /// 
    pub fn accept_connection(&mut self, (stream, addr): (TcpStream, SocketAddr)) -> SocketAddr {
        let (conn_term_tx, conn_term_rx) = oneshot::channel();
        self.conns.spawn(handle_tcp_stream(stream, HandleTcpStreamParams {
            event_tx: self.event_tx.clone(),
            term_rx: conn_term_rx
        }));
        self.conn_term_txs.push(conn_term_tx);
        addr
    }
    
    /// Closes this listener.
    ///
    /// It does so by sending the oneshot termination to all the tasks in join set and waits for them to finish
    /// by calling `join_next`.
    /// 
    /// `Returns`
    /// 
    /// A result containing on OK, or an error if something went wrong joining the connection tasks.
    /// 
    pub async fn close(mut self) -> Result<(), anyhow::Error> {
        for term_tx in self.conn_term_txs {
            let _ = term_tx.send(());
        }

        while let Some(result) = self.conns.join_next().await {
            result??;
        }
        
        Ok(())
    }
}

/// Listen for and handle TCP connections.
/// 
/// # Arguments
/// 
/// `options` - Options for configuring the TCP listener.
/// `params` - Parameters for constructing the TCP listener.
/// 
/// # Returns
/// 
/// A resulting containing nothing on OK, or an error if one occurred.
/// 
pub async fn listen_for_tcp_connections(options: TcpListenerOptions, params: TcpListenerParams) -> Result<(), anyhow::Error> {
    let TcpListenerOptions {
        host,
        port,
        signal_parent_ready
    } = options;
    let TcpListenerParams { 
        event_tx, 
        mut term_rx 
    } = params;
    
    let mut listener = MavrikTcpListener::bind(format!("{host}:{port}"), event_tx).await?;
    info!("Started TCP listener; accepting connections on {host}:{port}");
    
    if signal_parent_ready {
        signal_ready_to_parent_process();
    }

    let term_rx = &mut term_rx;
    
    // Continue accepting new connections until we receive the oneshot termination.
    loop {
        select! {
            conn = listener.listen() => {
                let addr = listener.accept_connection(conn?);
                info!("Accepted connection from {addr:?}");
            },
            result = term_rx.into_future() => {
                result?;
                debug!("[TCP] Received term");
                
                listener.close().await?;
                break;
            }
        }
    }
    
    info!("TCP listener stopped");
    Ok(())
}

fn signal_ready_to_parent_process() {
    match unsafe { kill(getppid(), SIGUSR1) } {
        0 => info!("Successfully signalled ready to parent process"),
        _ => warn!("Failed to send ready signal to parent process")
    }
}
