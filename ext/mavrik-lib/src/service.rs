use anyhow::Context;
use log::{debug, trace};
use std::fmt::Debug;
use std::future::{pending, Future, IntoFuture};
use tokio::select;
use tokio::sync::oneshot;

pub struct Service<F>
where
    F: Future<Output = Result<(), anyhow::Error>>,
{
    pub task: F,
    pub channel: ServiceChannel,
}

pub trait ServiceTask {
    type ReadyTask: Debug;

    async fn poll_task(&mut self) -> Self::ReadyTask {
        pending().await
    }

    #[allow(unused_variables)]
    async fn on_task_ready(&mut self, output: Self::ReadyTask) -> Result<(), anyhow::Error> {
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

pub struct Services;

impl Services {
    pub fn start<N, S>(
        name: N,
        mut service_task: S,
    ) -> Service<impl Future<Output = Result<(), anyhow::Error>>>
    where
        N: Into<String>,
        S: ServiceTask,
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

            let service_task = &mut service_task;
            let term_rx = &mut term_rx;
            loop {
                select! {
                    value = service_task.poll_task() => {
                        trace!(service = name, value:?; "Task ready");
                        service_task.on_task_ready(value).await.context(format!("{name}: task ready handling failed"))?;
                    },

                    result = term_rx.into_future() => {
                        result.context(format!("{name}: error receiving oneshot term"))?;

                        debug!(service = name; "Terminating");
                        service_task.on_terminate().await.context(format!("{name}: termination handling failed"))?;
                        break;
                    }
                }
            }

            debug!(service = name; "Completed");
            Result::<(), anyhow::Error>::Ok(())
        };

        Service { task, channel }
    }
}
