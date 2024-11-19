use std::fmt::Debug;
use std::future::{pending, Future, IntoFuture};
use anyhow::Context;
use log::{debug, trace};
use tokio::select;
use tokio::sync::oneshot;

pub trait MavrikService {
    type TaskOutput: Debug;

    async fn poll_task(&mut self) -> Self::TaskOutput {
        pending().await
    }

    #[allow(unused_variables)]
    async fn on_task_ready(&mut self, output: Self::TaskOutput) -> Result<(), anyhow::Error> {
        Ok(())
    }

    async fn on_terminate(&mut self) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

pub struct ServiceChannel {
    name: String,
    term_tx: Option<oneshot::Sender<()>>
}

impl ServiceChannel {
    pub fn terminate(&mut self) {
        if let Some(term_tx) = self.term_tx.take() {
            debug!("{}-channel: Sending termination signal", self.name);
            let _ = term_tx.send(());
        }
    }
}

pub fn start_service<N, S>(name: N, mut service: S) -> (impl Future<Output = Result<(), anyhow::Error>>, ServiceChannel)
where
    N: Into<String>,
    S: MavrikService
{
    let name = name.into();
    let (term_tx, mut term_rx) = oneshot::channel();
    let term_tx = Some(term_tx);
    let channel = ServiceChannel { name: name.clone(), term_tx };
    
    let task = async move {
        debug!(service = name; "Starting");
        
        let service = &mut service;
        let term_rx = &mut term_rx;
        loop {
            select! {
                value = service.poll_task() => {
                    trace!(service = name, value:?; "Task ready");
                    service.on_task_ready(value).await.context(format!("{name}: task ready handling failed"))?;
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
