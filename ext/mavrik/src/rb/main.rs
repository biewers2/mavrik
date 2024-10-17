use crate::signal_listener::listen_for_signals;
use crate::task_executor::execute_tasks;
use crate::tcp_listener::listen_for_tcp_connections;
use log::info;
use magnus::{function, Module, Object, RModule, Ruby};
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;

pub(crate) fn define_main(ruby: &Ruby) -> Result<(), magnus::Error> {
    let mavrik = ruby.class_object().const_get::<_, RModule>("Mavrik")?;
    mavrik.define_singleton_method("main", function!(main, 0))?;
    Ok(())
}

fn main() {
    rutie::Thread::call_without_gvl(
        move || {
            info!("Starting Maverik server");

            let (event_tx, event_rx) = mpsc::channel();

            let tcp_event_tx = event_tx.clone();
            let tcp_listener = thread::spawn(move || listen_for_tcp_connections(tcp_event_tx));

            let signal_event_tx = event_tx.clone();
            let signal_listener = thread::spawn(move || listen_for_signals(signal_event_tx));

            execute_tasks(event_tx, event_rx).expect("failure within task executor");

            join_thread(tcp_listener, "tcp listener");
            join_thread(signal_listener, "signal listener");
        },
        Some(|| {})
    );
}

fn join_thread<T>(handle: JoinHandle<Result<T, anyhow::Error>>, name: &str) {
    handle.join()
        .expect(&format!("failed to join {name} thread"))
        .expect(&format!("failure within {name} thread"));
}
