use anyhow::Context;
use libc::SIGINT;
use log::{debug, info};
use signal_hook_tokio::Signals;
use tokio::select;
use tokio::sync::{mpsc, oneshot};
use std::future::IntoFuture;
use futures::StreamExt;
use crate::events::MavrikEvent;

pub struct SignalListenerParams {
    pub(crate) event_tx: mpsc::Sender<MavrikEvent>,
    pub(crate) term_rx: oneshot::Receiver<()>
}

pub async fn listen_for_signals(params: SignalListenerParams) -> Result<(), anyhow::Error> {
    info!("Starting signal listener");
    let SignalListenerParams {
        event_tx,
        mut term_rx
    } = params;
    
    let mut signals = Signals::new(&[SIGINT]).context("failed to add signal listener")?;
    let handle = signals.handle();

    let term_rx = &mut term_rx;
    loop {
        select! {
            Some(signal) = signals.next() => {
                match signal {
                    SIGINT => event_tx.send(MavrikEvent::Signal(SIGINT)).await?,
                    _ => ()
                }
            },
            result = term_rx.into_future() => {
                debug!("[SIG] Received term");
                result?;
                break;
            }
        };
    }

    info!("Signal listener stopped");
    handle.close();
    Ok(())
}
