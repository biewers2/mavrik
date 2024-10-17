use std::process::exit;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use log::{debug, info, trace};
use magnus::{kwargs, Class, Module, RClass, RModule, Ruby, Symbol};
use magnus::value::ReprValue;
use crate::events::{MavrikEvent, ReadyThread, Task};
use crate::rb::module_mavrik;

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

    let execute_task = rb_new_execute_task(ruby)?;

    // Mark thread ready at the start.
    let ready_thread = ReadyThread { task_tx: task_tx.clone() };
    event_tx.send(MavrikEvent::ReadyThread(ready_thread)).expect("failed to send ready thread");

    while let Ok(task) = task_rx.recv() {
        let Task { definition, input_args, input_kwargs, .. } = task;
        
        info!("Executing '{definition}' with args '{input_args}' and kwargs '{input_kwargs}'");
        let ctx = kwargs!(
            "definition" => definition,
            "input_args" => input_args,
            "input_kwargs" => input_kwargs
        );
        let result = execute_task.funcall::<_, _, magnus::Value>("call", (ctx,))?;
        debug!("Result: {result:?}");

        let ready_thread = ReadyThread { task_tx: task_tx.clone() };
        event_tx.send(MavrikEvent::ReadyThread(ready_thread)).expect("failed to send ready thread");
    }

    Ok(())
}

fn rb_new_execute_task(ruby: &Ruby) -> Result<magnus::Value, magnus::Error> {
    let execute_task = module_mavrik(ruby)
        .const_get::<_, RClass>("ExecuteTask")?
        .new_instance(())?;

    Ok(execute_task)
}
