use crate::events::{GeneralEvent, MavrikEvent};
use crate::tcp::{MavrikTcpListener, TcpListenerOptions, TcpListenerParams};
use crate::service::start_service;
use crate::sig::{SignalListener, SignalListenerParams};
use crate::exe::{TaskExecutor, TaskExecutorOptions, TaskExecutorParams};
use log::{info, trace};
use tokio::sync::mpsc;
use tokio::{pin, try_join};
use crate::mem::TasksInMemory;

pub struct MavrikOptions {
    pub exe_options: TaskExecutorOptions,
    pub tcp_options: TcpListenerOptions
}

pub async fn start_event_loop(options: MavrikOptions) -> Result<(), anyhow::Error> {
    let (event_tx, mut event_rx) = mpsc::channel::<MavrikEvent>(1000);

    let task_memory = TasksInMemory::new();
    let params = TaskExecutorParams { task_memory, event_tx: event_tx.clone() };
    let exe = TaskExecutor::new(options.exe_options, params)?;
    let (exe_task, mut exe_chan) = start_service("EXE", exe);

    let params = TcpListenerParams { event_tx: event_tx.clone() };
    let tcp = MavrikTcpListener::new(options.tcp_options, params).await?;
    let (tcp_task, mut tcp_chan) = start_service("TCP", tcp);

    let params = SignalListenerParams { event_tx };
    let sig = SignalListener::new(params)?;
    let (sig_task, mut sig_chan) = start_service("SIG", sig);

    let event_loop_task = async move {
        while let Some(event) = event_rx.recv().await {
            trace!(event:?; "Received event in event loop");

            match event {
                MavrikEvent::General(GeneralEvent::Terminate) => {
                    info!("Received request for termination");
                    exe_chan.terminate();
                    tcp_chan.terminate();
                    sig_chan.terminate();
                },
                MavrikEvent::Exe(event) => exe_chan.send(event).await?,
                MavrikEvent::Tcp(event) => tcp_chan.send(event).await?,
                MavrikEvent::Sig(event) => sig_chan.send(event).await?
            }
        }

        info!("Event loop finished");
        Ok(())
    };

    pin!(event_loop_task);
    try_join!(event_loop_task, exe_task, tcp_task, sig_task)?;

    info!("Mavrik stopped");
    Ok(())
}
