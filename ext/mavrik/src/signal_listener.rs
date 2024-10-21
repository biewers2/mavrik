use crate::events::{GeneralEvent, MavrikEvent, SigEvent};
use crate::service::Service;
use futures::StreamExt;
use libc::{c_int, SIGINT};
use signal_hook_tokio::Signals;
use tokio::sync::mpsc;

pub struct SignalListenerParams {
    pub event_tx: mpsc::Sender<MavrikEvent>,
}

pub struct SignalListener {
    signals: Signals,
    events_tx: mpsc::Sender<MavrikEvent>
}

impl SignalListener {
    pub fn new(params: SignalListenerParams) -> Result<Self, anyhow::Error> {
        let signals = Signals::new(&[SIGINT])?;
        let events_tx = params.event_tx;
        
        Ok(Self { signals, events_tx })
    }
}

impl Service for SignalListener {
    type TaskOutput = Option<c_int>;
    type Message = SigEvent;

    async fn call_task(&mut self) -> Self::TaskOutput {
        self.signals.next().await
    }

    async fn on_task_ready(&mut self, signal: Self::TaskOutput) -> Result<(), anyhow::Error> {
        match signal {
            Some(SIGINT) => self.events_tx.send(MavrikEvent::General(GeneralEvent::Terminate)).await?,
            _ => ()
        }
        Ok(())
    }

    async fn on_message(&mut self, _message: Self::Message) -> Result<(), anyhow::Error> {
        Ok(())
    }

    async fn on_terminate(&mut self) -> Result<(), anyhow::Error> {
        self.signals.handle().close();
        Ok(())
    }
}
