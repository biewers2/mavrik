use std::process::exit;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use log::{debug, info, trace};
use magnus::{Class, Module, RClass, Ruby};
use magnus::value::ReprValue;
use crate::events::{MavrikEvent, Task};

#[derive(Debug)]
pub struct ReadyThread {
    task_tx: Sender<Task>
}

pub struct TaskExecutor {
    rb_threads: Vec<magnus::Thread>,
    task_buf: Vec<Task>,
    ready_thread_buf: Vec<ReadyThread>
}

impl TaskExecutor {
    pub fn new(event_tx: Sender<MavrikEvent>) -> Result<Self, anyhow::Error> {
        let rb_threads = rutie::Thread::call_with_gvl::<_, Result<Vec<magnus::Thread>, anyhow::Error>>(|| {
            let ruby = Ruby::get()?;

            let mut rb_threads = vec![];
            for _ in 0..5 {
                let thread_event_tx = event_tx.clone();
                rb_threads.push(ruby.thread_create_from_fn(move |r| rb_thread_main(r, thread_event_tx)));
            }
            Ok(rb_threads)
        })?;

        Ok(Self {
            rb_threads,
            task_buf: vec![],
            ready_thread_buf: vec![]
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

pub fn execute_tasks(event_tx: Sender<MavrikEvent>, event_rx: Receiver<MavrikEvent>) -> Result<(), anyhow::Error> {
    info!("Starting task executor");
    let mut executor = TaskExecutor::new(event_tx)?;

    while let Ok(value) = event_rx.recv() {
        debug!("Received event {value:?} in task executor");

        match value {
            MavrikEvent::Task(task) => executor.execute(task)?,
            MavrikEvent::ReadyThread(ready_thread) => executor.run_on_thread(ready_thread)?,
            MavrikEvent::Signal(sig) => exit(sig)
        }
    }
    Ok(())
}

fn rb_thread_main(ruby: &Ruby, event_tx: Sender<MavrikEvent>) -> Result<(), magnus::Error> {
    let (task_tx, task_rx) = mpsc::channel::<Task>();

    // Mark thread ready at the start.
    let ready_thread = ReadyThread { task_tx: task_tx.clone() };
    event_tx.send(MavrikEvent::ReadyThread(ready_thread)).expect("failed to send ready thread");

    while let Ok(task) = task_rx.recv() {
        let task_def = task.definition;
        let args = task.args;

        info!("Executing '{task_def}' with args '{args}'");
        let task_class = ruby.class_object().const_get::<_, RClass>(task_def)?;
        let result = task_class.new_instance(())?.funcall::<_, _, magnus::Value>("call", ())?;
        
        info!("Result: {result:?}");
        
        // ruby.eval::<magnus::Value>(&format!(r#"
        //     puts "hello?"
        //     task_class = Object.const_get({task_def})
        //     task_class.new.call({args})
        // "#)).expect("idk");

        let ready_thread = ReadyThread { task_tx: task_tx.clone() };
        event_tx.send(MavrikEvent::ReadyThread(ready_thread)).expect("failed to send ready thread");
    }

    Ok(())
}
