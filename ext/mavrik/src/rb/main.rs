use crate::event_loop::start_event_loop;
use crate::fetch;
use crate::rb::{mavrik_error, module_mavrik};
use crate::runtime::async_runtime;
use crate::signal_listener::{listen_for_signals, SignalListenerParams};
use crate::task_executor::{TaskExecutorOptions, TaskExecutorParams};
use crate::tcp_listener::{listen_for_tcp_connections, TcpServerOptions, TcpServerParams};
use log::{debug, info};
use magnus::{function, Object, RHash, Ruby};
use tokio::sync::{mpsc, oneshot};
use tokio::{join, pin, select};

pub(crate) fn define_main(ruby: &Ruby) -> Result<(), magnus::Error> {
    module_mavrik(ruby).define_singleton_method("main", function!(main, 1))?;
    Ok(())
}

fn main(options: RHash) -> Result<(), magnus::Error> {
    info!("Starting Mavrik server");
    debug!("Running with options {options:?}");

    let host = fetch!(options, :"host", "127.0.0.1".to_owned())?;
    let port = fetch!(options, :"port", 3001)?;
    let signal_parent_ready = fetch!(options, :"signal_parent_ready", false)?;
    let rb_thread_count = fetch!(options, :"thread_count", 4usize)?;

    rutie::Thread::call_without_gvl(move || async_runtime().block_on(async {
        // All events go to the task executor event loop through this MPSC channel.
        let (event_tx, event_rx) = mpsc::channel(1000);
        let (exe_term_tx, exe_term_rx) = oneshot::channel();
        let (tcp_term_tx, tcp_term_rx) = oneshot::channel();
        let (sig_term_tx, sig_term_rx) = oneshot::channel();

        // The task executor runs an event loop in this main thread. It handles all events from the process,
        // including scheduling of Ruby tasks to be executed on a Ruby thread.
        let exe_task = start_event_loop(
            TaskExecutorOptions {
                rb_thread_count
            },
            TaskExecutorParams {
                event_tx: event_tx.clone(),
                event_rx,
                term_rx: exe_term_rx,
            },
        );

        // TCP listener accepts connections from Ruby clients to send requests to.
        let tcp_task = listen_for_tcp_connections(
            TcpServerOptions {
                host: host.clone(),
                port,
                signal_parent_ready,
            },
            TcpServerParams {
                event_tx: event_tx.clone(),
                term_rx: tcp_term_rx,
            },
        );

        // Signals sent to this process are handled in this thread. Expected signals are captured and sent
        // as events to be handled by the event loop.
        let sig_task = listen_for_signals(SignalListenerParams {
            event_tx,
            term_rx: sig_term_rx,
        });

        pin!(exe_task);
        pin!(tcp_task);
        pin!(sig_task);

        // If one of these returns early, it's likely something went wrong.
        let result = select! {
            r = &mut exe_task => {
                debug!("EXE terminated, signalling term to TCP and SIG");
                let _ = tcp_term_tx.send(());
                let _ = sig_term_tx.send(());
                join!(&mut tcp_task, &mut sig_task);
                r
            },
            r = &mut tcp_task => {
                debug!("TCP terminated, signalling term to EXE and SIG");
                let _ = exe_term_tx.send(());
                let _ = sig_term_tx.send(());
                join!(&mut exe_task, &mut sig_task);
                r
            },
            r = &mut sig_task => {
                debug!("SIG terminated, signalling term to EXE and TCP");
                let _ = exe_term_tx.send(());
                let _ = tcp_term_tx.send(());
                join!(&mut exe_task, &mut tcp_task);
                r
            }
        };

        result.map_err(mavrik_error)
    }), Some(|| {}))?;

    info!("Mavrik server stopped");
    Ok(())
}
