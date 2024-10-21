use crate::event_loop::{start_event_loop, MavrikOptions};
use crate::io::TcpListenerOptions;
use crate::rb::{mavrik_error, module_mavrik};
use crate::runtime::async_runtime;
use crate::task_executor::TaskExecutorOptions;
use crate::{fetch, without_gvl};
use log::{debug, info};
use magnus::{function, Object, RHash, Ruby};

pub(crate) fn define_main(_ruby: &Ruby) -> Result<(), magnus::Error> {
    module_mavrik().define_singleton_method("main", function!(main, 1))?;
    Ok(())
}

fn main(options: RHash) -> Result<(), magnus::Error> {
    info!("Starting Mavrik server");
    debug!("Running with options {options:?}");

    let host = fetch!(options, :"host", "127.0.0.1".to_owned())?;
    let port = fetch!(options, :"port", 3001)?;
    let signal_parent_ready = fetch!(options, :"signal_parent_ready", false)?;
    let rb_thread_count = fetch!(options, :"thread_count", 4usize)?;

    without_gvl!({ 
        async_runtime().block_on(async {
            let options = MavrikOptions {
                exe_options: TaskExecutorOptions {
                    rb_thread_count
                },
                tcp_options: TcpListenerOptions {
                    host: host.clone(),
                    port,
                    signal_parent_ready,
                }
            };
            
            start_event_loop(options).await.map_err(mavrik_error)
        })
    })?;

    info!("Mavrik server stopped");
    Ok(())
}
