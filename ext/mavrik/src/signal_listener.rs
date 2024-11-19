use crate::service::MavrikService;
use futures::StreamExt;
use libc::{c_int, SIGINT};
use signal_hook_tokio::Signals;
use tokio::sync::oneshot;

pub struct SignalListener {
    signals: Signals,
    term_tx: Option<oneshot::Sender<()>>
}

impl SignalListener {
    pub fn new(term_tx: oneshot::Sender<()>) -> Result<Self, anyhow::Error> {
        let signals = Signals::new(&[SIGINT])?;
        
        Ok(Self { signals, term_tx: Some(term_tx)})
    }
}

impl MavrikService for SignalListener {
    type TaskOutput = Option<c_int>;
    type Message = ();

    async fn poll_task(&mut self) -> Self::TaskOutput {
        self.signals.next().await
    }

    async fn on_task_ready(&mut self, signal: Self::TaskOutput) -> Result<(), anyhow::Error> {
        match signal {
            Some(SIGINT) => if let Some(term_tx) = self.term_tx.take() {
                let _ = term_tx.send(());
            },
            
            _ => ()
        }
        Ok(())
    }

    async fn on_terminate(&mut self) -> Result<(), anyhow::Error> {
        self.signals.handle().close();
        Ok(())
    }
}
