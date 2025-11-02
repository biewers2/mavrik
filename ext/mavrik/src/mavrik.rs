use crate::executor::TaskExecutor;
use crate::service::Services;
use crate::signal_listener::SignalListener;
use crate::store::TasksInMemory;
use crate::tcp::MavrikTcpListener;
use anyhow::anyhow;
use log::info;
use magnus::{RHash, Symbol, TryConvert};
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::try_join;

pub struct Mavrik {
    options: MavrikOptions,
}

impl Mavrik {
    pub fn new(options: MavrikOptions) -> Self {
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

pub struct MavrikOptions(Arc<RHash>);

impl From<RHash> for MavrikOptions {
    fn from(value: RHash) -> Self {
        Self(Arc::new(value))
    }
}

impl MavrikOptions {
    pub fn get<T>(&self, key: &str, default: T) -> Result<T, anyhow::Error>
    where
        T: TryConvert,
    {
        self.rb_get(key, default).map_err(|e| anyhow!("{e}"))
    }

    fn rb_get<T>(&self, key: &str, default: T) -> Result<T, magnus::Error>
    where
        T: TryConvert,
    {
        if let Some(value) = self
            .0
            .fetch::<_, magnus::Value>(key)
            .map(|value| TryConvert::try_convert(value))
            .ok()
            .transpose()?
        {
            return Ok(value);
        }

        if let Some(value) = self
            .0
            .fetch::<_, magnus::Value>(Symbol::new(key))
            .map(|value| TryConvert::try_convert(value))
            .ok()
            .transpose()?
        {
            return Ok(value);
        }

        Ok(default)
    }
}
