use anyhow::Context;
use log::{debug, trace};
use std::fmt::Debug;
use std::future::{Future, IntoFuture, pending};
use tokio::select;
use tokio::sync::oneshot;

pub struct ServiceHandle<F>
where
    F: Future<Output = Result<(), anyhow::Error>>,
{
    pub task: F,
    pub channel: ServiceChannel,
}

pub trait Service {
    type Payload: Debug;

    async fn poll(&mut self) -> Self::Payload {
        pending().await
    }

    #[allow(unused_variables)]
    async fn on_ready(&mut self, output: Self::Payload) -> Result<(), anyhow::Error> {
        Ok(())
    }

    async fn on_terminate(&mut self) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

pub struct ServiceChannel {
    name: String,
    term_tx: Option<oneshot::Sender<()>>,
}

impl ServiceChannel {
    pub fn terminate(&mut self) {
        if let Some(term_tx) = self.term_tx.take() {
            debug!("{}-channel: Sending termination signal", self.name);
            let _ = term_tx.send(());
        }
    }
}

pub fn start_service<N, S>(
    name: N,
    mut service: S,
) -> ServiceHandle<impl Future<Output = Result<(), anyhow::Error>>>
where
    N: Into<String>,
    S: Service,
{
    let name = name.into();
    let (term_tx, mut term_rx) = oneshot::channel();
    let term_tx = Some(term_tx);
    let channel = ServiceChannel {
        name: name.clone(),
        term_tx,
    };

    let task = async move {
        debug!(service = name; "Starting");

        let service = &mut service;
        let term_rx = &mut term_rx;
        loop {
            select! {
                value = service.poll() => {
                    trace!(service = name, value:?; "Task ready");
                    service.on_ready(value).await.context(format!("{name}: task ready handling failed"))?;
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

    ServiceHandle { task, channel }
}
