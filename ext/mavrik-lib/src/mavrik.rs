use crate::executor::TaskExecutor;
use crate::service::Services;
use crate::signal_listener::SignalListener;
use crate::store::TasksInMemory;
use crate::tcp::MavrikTcpListener;
use log::info;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tokio::try_join;

pub struct Mavrik<'a> {
    options: &'a MavrikOptions,
}

impl<'a> Mavrik<'a> {
    pub fn new(options: &'a MavrikOptions) -> Self {
        Self { options }
    }

    pub async fn run(self) -> Result<(), anyhow::Error> {
        let (term_tx, term_rx) = oneshot::channel();
        let task_memory = TasksInMemory::new();

        let mut exe = Services::start(
            "EXE",
            TaskExecutor::new(&self.options, task_memory.clone())?,
        );
        let mut tcp = Services::start(
            "TCP",
            MavrikTcpListener::new(&self.options, task_memory).await?,
        );
        let mut sig = Services::start("SIG", SignalListener::new(term_tx)?);

        let exe_chan = &mut exe.channel;
        let tcp_chan = &mut tcp.channel;
        let sig_chan = &mut sig.channel;
        let cleanup_task = Box::pin(async move {
            let _ = term_rx.await?;
            info!("Received request for termination");
            exe_chan.terminate();
            tcp_chan.terminate();
            sig_chan.terminate();
            Ok(())
        });

        try_join!(exe.task, tcp.task, sig.task, cleanup_task)?;
        info!("Mavrik stopped");
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MavrikOptions {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub rb_thread_count: Option<usize>,
    pub signal_parent_ready: Option<bool>,
}
