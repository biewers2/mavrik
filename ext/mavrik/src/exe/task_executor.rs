use crate::events::{ExeEvent, MavrikEvent, Task, TaskId, TaskResult};
use crate::exe::chan::{ExecutorChannel, ThreadMessage};
use crate::exe::thread_main::rb_thread_main;
use crate::rb::in_ruby;
use crate::service::Service;
use anyhow::{anyhow, Context};
use log::{debug, error, trace};
use magnus::value::ReprValue;
use std::collections::HashMap;
use tokio::sync::mpsc;
use crate::mem::TaskMemory;

/// ID associated with a Ruby thread created by the task executor.
pub type ThreadId = usize;

/// Options for configuring the task executor.
pub struct TaskExecutorOptions {
    // Number of Ruby threads to create.
    pub rb_thread_count: usize,
}

/// Parameters for creating the task executor.
pub struct TaskExecutorParams<M>
where
    M: TaskMemory
{
    /// Where to send events to.
    pub event_tx: mpsc::Sender<MavrikEvent>,
    
    pub task_memory: M
}

/// Entry in the thread table.
///
/// This is useful for associating threads to other data structures, such as the task sender used to send tasks to the
/// thread when received.
///
struct ThreadTableEntry {
    /// The associated thread.
    thread: magnus::Thread,

    /// Where to send tasks to for this thread to execute.
    task_tx: mpsc::Sender<(TaskId, Task)>
}

/// Task executor service, used to execute tasks concurrently in Ruby threads.
pub struct TaskExecutor<M>
where
    M: TaskMemory
{
    /// Where tasks and task results are stored and buffered.
    task_memory: M,
    
    /// Table mapping thread IDs to threads and associated structs.
    thread_table: HashMap<ThreadId, ThreadTableEntry>,

    /// Where to receive messages from threads.
    thread_rx: mpsc::Receiver<ThreadMessage>,

    /// List of threads available to process tasks listed by their IDs.
    ready_buf: Vec<ThreadId>
}

impl<M> TaskExecutor<M>
where
    M: TaskMemory
{
    /// Create a new task executor service.
    ///
    /// # Arguments
    ///
    /// `options` - Options for configuring the task executor.
    /// `params` - Values for constructing the task executor.
    ///
    pub fn new(options: TaskExecutorOptions, params: TaskExecutorParams<M>) -> Result<Self, anyhow::Error> {
        let TaskExecutorOptions { rb_thread_count, .. } = options;
        let TaskExecutorParams { task_memory, .. } = params;

        let (thread_tx, thread_rx) = mpsc::channel(10);
        let thread_table = in_ruby(|r| {
            let mut table = HashMap::new();
            for thread_id in 0..rb_thread_count {
                let (task_tx, task_rx) = mpsc::channel(1);
                let chan = ExecutorChannel::new(task_rx, thread_tx.clone());

                let thread = r.thread_create_from_fn(move |r| rb_thread_main(r, thread_id, chan));
                table.insert(thread_id, ThreadTableEntry { thread, task_tx });
            }
            table
        });
        
        Ok(Self {
            task_memory,
            thread_table,
            thread_rx,
            ready_buf: vec![],
        })
    }

    pub async fn execute_task(&mut self, task: Task) -> Result<TaskId, anyhow::Error> {
        let task_id = self.task_memory.push_queue(task).await.context("pushing task to memory")?;
        if let Some(thread_id) = self.ready_buf.pop() {
            self.execute_on_thread(thread_id).await.context("executing tasks on thread")?;
        }
        Ok(task_id)
    }

    pub async fn execute_on_thread(&mut self, thread_id: ThreadId) -> Result<(), anyhow::Error> {
        let result = self.task_memory.pop_queue().await.context("popping task off memory queue")?;
        match result {
            Some((task_id, task)) => self.run_task_on_thread(thread_id, task_id, task).await?,
            None => {
                trace!("No tasks in queue, pushing thread on to queue");
                self.ready_buf.push(thread_id);
            }
        }
        Ok(())
    }

    async fn run_task_on_thread(&self, thread_id: ThreadId, task_id: TaskId, task: Task) -> Result<(), anyhow::Error> {
        trace!(thread_id, task:?; "Running on thread");

        let entry = self.thread_table.get(&thread_id).ok_or(anyhow!("thread with ID {thread_id} not found"))?;
        entry.task_tx.send((task_id, task)).await.context("sending task to thread")?;

        Ok(())
    }
}

impl<M> Service for TaskExecutor<M>
where
    M: TaskMemory
{
    type TaskOutput = Option<ThreadMessage>;
    type Message = ExeEvent;

    async fn poll_task(&mut self) -> Self::TaskOutput {
        self.thread_rx.recv().await
    }

    async fn on_task_ready(&mut self, message: Self::TaskOutput) -> Result<(), anyhow::Error> {
        match message {
            Some(ThreadMessage::ThreadReady(thread_id)) => {
                self.execute_on_thread(thread_id).await.context("executing tasks on thread")?;
                Ok(())
            },

            Some(ThreadMessage::Completed { task_id, task_result }) => {
                trace!(task_id:?, task_result:?; "Task completed");
                self.task_memory.insert_completed(task_id, task_result).await.context("adding task to completed memory")?;
                Ok(())
            },

            None => Err(anyhow!("Thread messaging channel unexpectedly closed early"))
        }
    }

    async fn on_message(&mut self, message: Self::Message) -> Result<(), anyhow::Error> {
        match message {
            ExeEvent::NewTask {
                task,
                value_tx
            } => {
                let task_id = self.execute_task(task).await.context("executing new task from message")?;
                if let Err(task_id) = value_tx.send(task_id) {
                    error!(task_id; "Could not send task ID over oneshot channel");
                }
            },

            ExeEvent::AwaitTask {
                task_id,
                value_tx
            } => {
                match self.task_memory.remove_completed(task_id).await.context("getting completed task")? {
                    // Task completed, send result to caller.
                    Some((task_id, task_result)) => {
                        // Put the value back in the buffer if it failed to send.
                        if let Err(task_result) = value_tx.send(task_result) {
                            self.task_memory.insert_completed(task_id, task_result).await.context("adding completed task back to memory")?;
                        }
                    },

                    // No task w/ ID found, send failure back.
                    None => {
                        let failure = TaskResult::from(anyhow!("no task w/ ID {task_id} found"));
                        let _ = value_tx.send(failure);
                    }
                }
            }
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
                { entry.task_tx; } // drop TX

                debug!(thread_id; "Joining thread");
                let result: Result<magnus::Value, magnus::Error> = entry.thread.funcall_public("join", (30,));
                if let Err(e) = result {
                    error!(thread_id, e:?; "Could not join thread");
                }
            }
        }

        Ok(())
    }
}
