use crate::fetch;
use crate::signal_listener::listen_for_signals;
use crate::task_executor::{execute_tasks, TaskExecutorOptions};
use crate::tcp_listener::{listen_for_tcp_connections, TcpServerOptions};
use libc::exit;
use log::{debug, info};
use magnus::{function, Module, Object, RHash, RModule, Ruby};
use std::sync::mpsc;
use std::thread;

pub(crate) fn define_main(ruby: &Ruby) -> Result<(), magnus::Error> {
    let mavrik = ruby.class_object().const_get::<_, RModule>("Mavrik")?;
    mavrik.define_singleton_method("main", function!(main, 1))?;
    Ok(())
}

fn main(options: RHash) -> Result<(), magnus::Error> {
    debug!("options: {options:?}");
    
    let host = fetch!(options, :"host", "127.0.0.1".to_owned())?;
    let port = fetch!(options, :"port", 3001)?;
    let signal_parent_ready = fetch!(options, "signal_parent_ready", false)?;
    let rb_thread_count = fetch!(options, "thread_count", 4usize)?;
    
    debug!("Running Mavrik on {host}:{port} with {rb_thread_count} threads; Signal parent when ready? {signal_parent_ready}");
    
    rutie::Thread::call_without_gvl(
        move || {
            info!("Starting Maverik server");

            // All events go to the task executor event loop through this MPSC channel.
            let (event_tx, event_rx) = mpsc::channel();

            // TCP listener accepts connections from Ruby clients to send requests to.
            let tcp_event_tx = event_tx.clone();
            let options = TcpServerOptions { host: host.clone(), port, signal_parent_ready };
            let _tcp_listener = thread::spawn(move || listen_for_tcp_connections(options, tcp_event_tx));

            // Signals sent to this process are handled in this thread. Expected signals are captured and sent
            // as events to be handled by the event loop.
            let signal_event_tx = event_tx.clone();
            let _signal_listener = thread::spawn(move || listen_for_signals(signal_event_tx));

            // The task executor runs an event loop in this main thread. It handles all events from the process,
            // including scheduling of Ruby tasks to be executed on a Ruby thread.
            let options = TaskExecutorOptions { rb_thread_count };
            execute_tasks(options, event_tx, event_rx).expect("failure within task executor");

            // TODO - implement controlled exit
            unsafe { exit(0) };

            // join_thread(tcp_listener, "tcp listener");
            // join_thread(signal_listener, "signal listener");
        },
        Some(|| {})
    );

    Ok(())
}

// fn join_thread<T>(handle: JoinHandle<Result<T, anyhow::Error>>, name: &str) {
//     handle.join()
//         .expect(&format!("failed to join {name} thread"))
//         .expect(&format!("failure within {name} thread"));
// }
