use crate::events::{MavrikEvent, Task};
use crate::rb::{in_ruby, mavrik_error, module_mavrik};
use crate::runtime::async_runtime;
use log::trace;
use magnus::value::ReprValue;
use magnus::{kwargs, Class, Module, RClass, Ruby};
use std::collections::HashMap;
use anyhow::anyhow;
use tokio::sync::{mpsc, oneshot};
use crate::{with_gvl, without_gvl};

pub struct TaskExecutorOptions {
    pub rb_thread_count: usize,
}

pub struct TaskExecutorParams {
    pub event_tx: mpsc::Sender<MavrikEvent>,
    pub event_rx: mpsc::Receiver<MavrikEvent>,
    pub term_rx: oneshot::Receiver<()>
}

pub type ThreadId = usize;

struct ThreadTableEntry {
    thread: magnus::Thread,
    task_tx: mpsc::Sender<Task>
}

pub struct TaskExecutor {
    thread_table: HashMap<usize, ThreadTableEntry>,
    task_buf: Vec<Task>,
    ready_buf: Vec<ThreadId>,
}

impl TaskExecutor {
    pub fn new(options: TaskExecutorOptions, event_tx: mpsc::Sender<MavrikEvent>) -> Result<Self, anyhow::Error> {
        let TaskExecutorOptions { rb_thread_count } = options;

        let thread_table = with_gvl!({
            in_ruby::<Result<_, anyhow::Error>>(|ruby| {
                let mut table = HashMap::new();
                for i in 0..rb_thread_count {
                    let thread_id = i;
                    let (task_tx, task_rx) = mpsc::channel::<Task>(1000);
                    let event_tx = event_tx.clone();
                    let thread = ruby.thread_create_from_fn(move |r| rb_thread_main(r, thread_id, event_tx, task_rx));
                    table.insert(thread_id, ThreadTableEntry { thread, task_tx });
                }
                
                Ok(table)
            })
        })?;

        Ok(Self {
            thread_table,
            task_buf: vec![],
            ready_buf: vec![],
        })
    }

    pub async fn execute(&mut self, task: Task) -> Result<(), anyhow::Error> {
        match self.ready_buf.pop() {
            Some(thread_id) => self.run_task_on_thread(thread_id, task).await?,
            None => {
                trace!("No ready threads, pushing task on to queue");
                self.task_buf.push(task);
            }
        }
        Ok(())
    }

    pub async fn run_on_thread(&mut self, thread_id: ThreadId) -> Result<(), anyhow::Error> {
        match self.task_buf.pop() {
            Some(task) => self.run_task_on_thread(thread_id, task).await?,
            None => {
                trace!("No tasks in queue, pushing thread on to queue");
                self.ready_buf.push(thread_id);
            }
        }
        Ok(())
    }
    
    pub fn into_threads(self) -> Vec<magnus::Thread> {
        self.thread_table.into_values().map(|e| e.thread).collect()
    }

    async fn run_task_on_thread(&self, thread_id: ThreadId, task: Task) -> Result<(), anyhow::Error> {
        trace!("Running {task:?} on thread {thread_id}");
        
        let entry = self.thread_table.get(&thread_id).ok_or(anyhow!("thread with ID {thread_id} not found"))?;
        entry.task_tx.send(task).await?;
        
        Ok(())
    }
}

fn rb_thread_main(_ruby: &Ruby, thread_id: ThreadId, event_tx: mpsc::Sender<MavrikEvent>, mut task_rx: mpsc::Receiver<Task>) -> Result<(), magnus::Error> {
    let execute_task = module_mavrik()
        .const_get::<_, RClass>("ExecuteTask")?
        .new_instance(())?;

    without_gvl!({
        async_runtime().block_on(async {
            // Mark thread ready at the start.
            event_tx.send(MavrikEvent::ThreadReady(thread_id)).await.map_err(mavrik_error)?;

            while let Some(task) = task_rx.recv().await {
                let Task { id, definition, input_args, input_kwargs, .. } = &task;
                trace!("({id}) Executing '{definition}' with args '{input_args}' and kwargs '{input_kwargs}'");

                let result = with_gvl!({
                    let ctx = kwargs!(
                        "definition" => definition.as_str(),
                        "input_args" => input_args.as_str(),
                        "input_kwargs" => input_kwargs.as_str()
                    );
                    let result = execute_task.funcall::<_, _, magnus::Value>("call", (ctx,))?;
                    Result::<_, magnus::Error>::Ok(result)
                });

                trace!("({id}) Result of execution: {result:?}");

                event_tx.send(MavrikEvent::ThreadReady(thread_id)).await.map_err(mavrik_error)?;
            }

            Ok(())
        })
    })
}
