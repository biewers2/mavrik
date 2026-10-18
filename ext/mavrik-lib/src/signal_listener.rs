use crate::service::Service;
use futures::StreamExt;
use libc::{SIGINT, c_int};
use signal_hook_tokio::Signals;
use tokio::sync::oneshot;

pub struct SignalListener {
    signals: Signals,
    term_tx: Option<oneshot::Sender<()>>,
}

impl SignalListener {
    pub fn new(term_tx: oneshot::Sender<()>) -> Result<Self, anyhow::Error> {
        let signals = Signals::new(&[SIGINT])?;

        Ok(Self {
            signals,
            term_tx: Some(term_tx),
        })
    }
}

impl Service for SignalListener {
    type Payload = Option<c_int>;

    async fn poll(&mut self) -> Self::Payload {
        self.signals.next().await
    }

    async fn on_ready(&mut self, signal: Self::Payload) -> Result<(), anyhow::Error> {
        match signal {
            Some(SIGINT) => {
                if let Some(term_tx) = self.term_tx.take() {
                    let _ = term_tx.send(());
                }
            }

            _ => (),
        }
        Ok(())
    }

    async fn on_terminate(&mut self) -> Result<(), anyhow::Error> {
        self.signals.handle().close();
        Ok(())
    }
}
