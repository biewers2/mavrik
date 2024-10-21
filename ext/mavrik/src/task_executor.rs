use crate::events::{ExeEvent, MavrikEvent, Task};
use crate::rb::{in_ruby, mavrik_error, module_mavrik};
use crate::runtime::async_runtime;
use log::{debug, trace};
use magnus::value::ReprValue;
use magnus::{kwargs, Class, Module, RClass, Ruby};
use std::collections::HashMap;
use anyhow::anyhow;
use tokio::sync::mpsc;
use crate::{with_gvl, without_gvl};
use crate::service::Service;

type ThreadId = usize;

pub struct TaskExecutorOptions {
    pub rb_thread_count: usize,
}

pub struct TaskExecutorParams {
    pub event_tx: mpsc::Sender<MavrikEvent>
}

struct ThreadTableEntry {
    thread: magnus::Thread,
    task_tx: mpsc::Sender<Task>
}

pub struct TaskExecutor {
    thread_table: HashMap<usize, ThreadTableEntry>,
    thread_ready_rx: mpsc::Receiver<ThreadId>,
    task_buf: Vec<Task>,
    ready_buf: Vec<ThreadId>,
}

impl TaskExecutor {
    pub fn new(options: TaskExecutorOptions, params: TaskExecutorParams) -> Result<Self, anyhow::Error> {
        let TaskExecutorOptions { rb_thread_count } = options;
        let TaskExecutorParams { .. } = params;

        let (thread_ready_tx, thread_ready_rx) = mpsc::channel(100);

        let thread_table = with_gvl!({
            in_ruby::<Result<_, anyhow::Error>>(|ruby| {
                let mut table = HashMap::new();
                for i in 0..rb_thread_count {
                    let thread_id = i;
                    let (task_tx, task_rx) = mpsc::channel::<Task>(1000);
                    let thread_ready_tx = thread_ready_tx.clone();
                    let thread = ruby.thread_create_from_fn(move |r| rb_thread_main(r, thread_id, thread_ready_tx, task_rx));
                    table.insert(thread_id, ThreadTableEntry { thread, task_tx });
                }
        
                Ok(table)
            })
        })?;

        Ok(Self {
            thread_table,
            thread_ready_rx,
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

    async fn run_task_on_thread(&self, thread_id: ThreadId, task: Task) -> Result<(), anyhow::Error> {
        trace!("Running {task:?} on thread {thread_id}");

        let entry = self.thread_table.get(&thread_id).ok_or(anyhow!("thread with ID {thread_id} not found"))?;
        entry.task_tx.send(task).await?;

        Ok(())
    }
}

impl Service for TaskExecutor {
    type TaskOutput = Option<ThreadId>;
    type Message = ExeEvent;

    async fn call_task(&mut self) -> Self::TaskOutput {
        self.thread_ready_rx.recv().await
    }

    async fn on_task_ready(&mut self, thread_id: Self::TaskOutput) -> Result<(), anyhow::Error> {
        match thread_id {
            Some(thread_id) => {
                match self.task_buf.pop() {
                    Some(task) => self.run_task_on_thread(thread_id, task).await?,
                    None => {
                        trace!("No tasks in queue, pushing thread on to queue");
                        self.ready_buf.push(thread_id);
                    }
                }
                Ok(())
            },
            None => Ok(()) // Err(anyhow!("Thread ready channel unexpectedly closed early"))
        }
    }

    async fn on_message(&mut self, message: Self::Message) -> Result<(), anyhow::Error> {
        match message {
            ExeEvent::NewTask(task) => self.execute(task).await?
        };
        Ok(())
    }

    async fn on_terminate(&mut self) -> Result<(), anyhow::Error> {
        let thread_ids = self.thread_table
            .keys()
            .map(|u| u.to_owned())
            .collect::<Vec<usize>>();
        
        for thread_id in thread_ids {
            if let Some(entry) = self.thread_table.remove(&thread_id) {
                debug!("Killing thread {}", thread_id);
                let _ = entry.thread.kill();
            }
        }
        
        Ok(())
    }
}

fn rb_thread_main(_ruby: &Ruby, thread_id: ThreadId, thread_ready_tx: mpsc::Sender<ThreadId>, mut task_rx: mpsc::Receiver<Task>) -> Result<(), magnus::Error> {
    let execute_task = module_mavrik()
        .const_get::<_, RClass>("ExecuteTask")?
        .new_instance(())?;

    without_gvl!({
        async_runtime().block_on(async {
            // Mark thread ready at the start.
            thread_ready_tx.send(thread_id).await.map_err(mavrik_error)?;

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

                thread_ready_tx.send(thread_id).await.map_err(mavrik_error)?;
            }

            Ok(())
        })
    })
}
