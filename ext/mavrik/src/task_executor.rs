use crate::events::{MavrikEvent, ReadyThread, Task};
use crate::rb::class_execute_task_new;
use log::{debug, info, trace};
use magnus::value::ReprValue;
use magnus::{kwargs, Ruby};
use std::process::exit;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

pub struct TaskExecutorOptions {
    pub rb_thread_count: usize,
}

pub struct TaskExecutor {
    _rb_threads: Vec<magnus::Thread>,
    task_buf: Vec<Task>,
    ready_thread_buf: Vec<ReadyThread>,
}

impl TaskExecutor {
    pub fn new(options: TaskExecutorOptions, event_tx: Sender<MavrikEvent>) -> Result<Self, anyhow::Error> {
        let TaskExecutorOptions { rb_thread_count } = options;
        
        let rb_threads = rutie::Thread::call_with_gvl::<_, Result<Vec<magnus::Thread>, anyhow::Error>>(|| {
            let ruby = Ruby::get()?;

            let mut rb_threads = vec![];
            for _ in 0..rb_thread_count {
                let thread_event_tx = event_tx.clone();
                rb_threads.push(ruby.thread_create_from_fn(move |r| rb_thread_main(r, thread_event_tx)));
            }
            Ok(rb_threads)
        })?;

        Ok(Self {
            _rb_threads: rb_threads,
            task_buf: vec![],
            ready_thread_buf: vec![],
        })
    }

    pub fn execute(&mut self, task: Task) -> Result<(), anyhow::Error> {
        match self.ready_thread_buf.pop() {
            Some(ready_thread) => self.run_task_on_thread(ready_thread, task)?,
            None => {
                trace!("No ready threads, pushing {task:?} onto queue");
                self.task_buf.push(task);
            }
        }
        Ok(())
    }

    pub fn run_on_thread(&mut self, ready_thread: ReadyThread) -> Result<(), anyhow::Error> {
        match self.task_buf.pop() {
            Some(task) => self.run_task_on_thread(ready_thread, task)?,
            None => {
                trace!("No tasks in queue, pushing thread onto queue");
                self.ready_thread_buf.push(ready_thread);
            }
        }
        Ok(())
    }

    fn run_task_on_thread(&self, ready_thread: ReadyThread, task: Task) -> Result<(), anyhow::Error> {
        debug!("Running {task:?} on thread");
        ready_thread.task_tx.send(task)?;
        Ok(())
    }
}

pub fn execute_tasks(options: TaskExecutorOptions, event_tx: Sender<MavrikEvent>, event_rx: Receiver<MavrikEvent>) -> Result<(), anyhow::Error> {
    info!("Starting task executor");
    let mut executor = TaskExecutor::new(options, event_tx)?;

    while let Ok(value) = event_rx.recv() {
        trace!("Received event {value:?} in task executor");

        match value {
            MavrikEvent::NewTask(task) => executor.execute(task)?,
            MavrikEvent::ReadyThread(ready_thread) => executor.run_on_thread(ready_thread)?,
            MavrikEvent::Signal(libc::SIGINT | libc::SIGTERM) => {
                info!("Received request for termination, stopping threads...");
                break
            },
            MavrikEvent::Signal(sig) => exit(sig)
        }
    }
    Ok(())
}

fn rb_thread_main(ruby: &Ruby, event_tx: Sender<MavrikEvent>) -> Result<(), magnus::Error> {
    let (task_tx, task_rx) = mpsc::channel::<Task>();

    let execute_task = class_execute_task_new(ruby)?;

    // Mark thread ready at the start.
    let ready_thread = ReadyThread { task_tx: task_tx.clone() };
    event_tx.send(MavrikEvent::ReadyThread(ready_thread)).expect("failed to send ready thread");

    rutie::Thread::call_without_gvl(
        move || {
            while let Ok(task) = task_rx.recv() {
                let Task { id, definition, input_args, input_kwargs, .. } = &task;
                debug!("({id}) Executing '{definition}' with args '{input_args}' and kwargs '{input_kwargs}'");
                
                let result: Result<magnus::Value, magnus::Error> = rutie::Thread::call_with_gvl(move || {
                    let ctx = kwargs!(
                        "definition" => definition.as_str(),
                        "input_args" => input_args.as_str(),
                        "input_kwargs" => input_kwargs.as_str()
                    );
                    let result = execute_task.funcall::<_, _, magnus::Value>("call", (ctx,))?;
                    Ok(result)
                });
                
                debug!("({id}) Result of execution: {result:?}");

                let ready_thread = ReadyThread { task_tx: task_tx.clone() };
                event_tx.send(MavrikEvent::ReadyThread(ready_thread)).expect("failed to send ready thread");
            }

            Ok(())
        },
        Some(|| {}),
    )
}
