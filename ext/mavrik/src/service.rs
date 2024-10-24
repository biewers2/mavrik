use std::fmt::Debug;
use std::future::{Future, IntoFuture};
use anyhow::Context;
use log::{debug, trace};
use tokio::select;
use tokio::sync::{mpsc, oneshot};

pub trait Service {
    type TaskOutput: Debug;
    type Message: Debug;

    async fn poll_task(&mut self) -> Self::TaskOutput;

    async fn on_task_ready(&mut self, output: Self::TaskOutput) -> Result<(), anyhow::Error>;

    #[allow(unused_variables)]
    async fn on_message(&mut self, message: Self::Message) -> Result<(), anyhow::Error> {
        Ok(())
    }

    async fn on_terminate(&mut self) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

pub struct ServiceChannel<M> {
    name: String,
    message_tx: mpsc::Sender<M>,
    term_tx: Option<oneshot::Sender<()>>
}

impl<M> ServiceChannel<M>
where
    M: Send + Sync + 'static
{
    pub async fn send(&self, message: M) -> Result<(), anyhow::Error> {
        let name = &self.name;
        self.message_tx.send(message).await.context(format!("{name}-channel: failed to message service"))?;
        Ok(())
    }

    pub fn terminate(&mut self) {
        if let Some(term_tx) = self.term_tx.take() {
            debug!("{}-channel: Sending termination signal", self.name);
            let _ = term_tx.send(());
        }
    }
}

pub fn start_service<N, S>(name: N, mut service: S) -> (impl Future<Output = Result<(), anyhow::Error>>, ServiceChannel<S::Message>)
where
    N: Into<String>,
    S: Service
{
    let name = name.into();
    let (term_tx, mut term_rx) = oneshot::channel();
    let term_tx = Some(term_tx);
    let (message_tx, mut message_rx) = mpsc::channel(100);
    let channel = ServiceChannel { name: name.clone(), message_tx, term_tx };
    
    let task = async move {
        debug!(service = name; "Starting");
        
        let service = &mut service;
        let message_rx = &mut message_rx;
        let term_rx = &mut term_rx;
        loop {
            select! {
                value = service.poll_task() => {
                    trace!(service = name, value:?; "Task ready");
                    service.on_task_ready(value).await.context(format!("{name}: task ready handling failed"))?;
                },

                Some(message) = message_rx.recv() => {
                    trace!(service = name, message:?; "Received message");
                    service.on_message(message).await.context(format!("{name}: message handling failed"))?;
                },

                result = term_rx.into_future() => {
                    result.context(format!("{name}: error receiving oneshot term"))?;
                        
                    debug!(service = name; "Terminating");
                    service.on_terminate().await.context(format!("{name}: termination handling failed"))?;
                    break;
                }
            }
        }

        debug!(service = name; "Completed");
        Result::<(), anyhow::Error>::Ok(())
    };

    (task, channel)
}
