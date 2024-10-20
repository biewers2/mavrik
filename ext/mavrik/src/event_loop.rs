use crate::events::MavrikEvent;
use crate::task_executor::{TaskExecutor, TaskExecutorOptions, TaskExecutorParams};
use log::{debug, info, trace};
use std::future::IntoFuture;
use std::process::exit;
use tokio::select;

pub async fn start_event_loop(options: TaskExecutorOptions, params: TaskExecutorParams) -> Result<(), anyhow::Error> {
    info!("Starting task executor");
    let TaskExecutorParams {
        event_tx,
        mut event_rx,
        mut term_rx
    } = params;

    let mut executor = TaskExecutor::new(options, event_tx)?;

    let term_rx = &mut term_rx;
    loop {
        select! {
            Some(value) = event_rx.recv() => {
                trace!("Received event {value:?} in task executor");

                match value {
                    MavrikEvent::NewTask(task) => executor.execute(task).await?,
                    MavrikEvent::ThreadReady(ready_thread) => executor.run_on_thread(ready_thread).await?,
                    MavrikEvent::Signal(libc::SIGINT | libc::SIGTERM) => {
                        info!("Received request for termination");
                        break;
                    },
                    MavrikEvent::Signal(sig) => exit(sig)
                }
            },
            result = term_rx.into_future() => {
                debug!("[EXE] Received term");
                result?;
                break
            }
        }
    }

    for thread in executor.into_threads() {
        debug!("Killing thread {thread:?}");
        // let _ = thread.kill();
    }
    
    Ok(())
}
