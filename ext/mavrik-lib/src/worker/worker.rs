use crate::messaging::{Task, TaskId, TaskResult};
use crate::rb::util::in_ruby;
use crate::service::ServiceTask;
use crate::store::ProcessStore;
use crate::worker::thread_main::rb_thread_main;
use log::{debug, error, warn};
use magnus::error::ErrorType;
use magnus::value::ReprValue;
use ruby_bridge::with_gvl;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::select;
use tokio::sync::mpsc;

/// ID associated with a Ruby thread created by the task executor.
pub type ThreadId = usize;

#[derive(Debug)]
pub enum ThreadMessage {
    ThreadReady(ThreadId),
    TaskComplete((TaskId, TaskResult)),
}

#[derive(Debug)]
pub enum TaskOutputKind {
    ThreadReady(ThreadId),
    TaskComplete((TaskId, TaskResult)),
    NextTask((TaskId, Task)),
}

impl From<ThreadMessage> for TaskOutputKind {
    fn from(value: ThreadMessage) -> Self {
        match value {
            ThreadMessage::ThreadReady(id) => TaskOutputKind::ThreadReady(id),
            ThreadMessage::TaskComplete((id, result)) => TaskOutputKind::TaskComplete((id, result)),
        }
    }
}

pub struct ThreadTableEntry {
    thread: magnus::Thread,
    task_tx: mpsc::Sender<(TaskId, Task)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerOptions {
    pub rb_thread_count: Option<usize>,
}

impl WorkerOptions {
    pub fn get_rb_thread_count(&self) -> usize {
        self.rb_thread_count.unwrap_or(4)
    }
}

pub struct Worker<Store> {
    store: Store,
    thread_table: HashMap<ThreadId, ThreadTableEntry>,
    messages_rx: mpsc::Receiver<ThreadMessage>,
    task_buf: Vec<(TaskId, Task)>,
    thread_ready_buf: Vec<ThreadId>,
}

impl<Store> Worker<Store> {
    /// Create a new task executor service.
    ///
    /// # Arguments
    ///
    /// `options` - Options for configuring the task executor.
    /// `params` - Values for constructing the task executor.
    ///
    pub fn new(options: &WorkerOptions, store: Store) -> Result<Self, anyhow::Error> {
        let rb_thread_count = options.get_rb_thread_count();

        let mut thread_table = HashMap::new();
        let (messages_tx, messages_rx) = mpsc::channel(rb_thread_count);
        let task_buf = Vec::new();
        let thread_ready_buf = Vec::new();

        in_ruby(|ruby| {
            for thread_id in 0..rb_thread_count {
                let (task_tx, task_rx) = mpsc::channel(1);
                let messages_tx = messages_tx.clone();
                let thread = ruby.thread_create_from_fn(move |r| {
                    rb_thread_main(r, thread_id, messages_tx, task_rx)
                });

                thread_table.insert(thread_id, ThreadTableEntry { thread, task_tx });
            }
        });

        Ok(Self {
            store,
            thread_table,
            messages_rx,
            task_buf,
            thread_ready_buf,
        })
    }
}

impl<Store> ServiceTask for Worker<Store>
where
    Store: ProcessStore<Id = TaskId, Error = anyhow::Error>,
{
    type ReadyTask = Result<TaskOutputKind, anyhow::Error>;

    async fn poll_task(&mut self) -> Self::ReadyTask {
        select! {
            result = self.store.pull_next_input(), if self.task_buf.len() < 100 => {
                Ok(TaskOutputKind::NextTask(result?))
            },

            Some(message) = self.messages_rx.recv() => {
                Ok(message.into())
            }
        }
    }

    async fn on_task_ready(&mut self, output: Self::ReadyTask) -> Result<(), anyhow::Error> {
        match output? {
            TaskOutputKind::NextTask((task_id, task)) => {
                match self.thread_ready_buf.pop() {
                    Some(thread_id) => {
                        let entry = self.thread_table.get(&thread_id).expect("thread not found");
                        entry.task_tx.send((task_id, task)).await?;
                    }
                    None => {
                        self.task_buf.push((task_id, task));
                    }
                }
                Ok(())
            }

            TaskOutputKind::ThreadReady(thread_id) => {
                match self.task_buf.pop() {
                    Some((task_id, task)) => {
                        let entry = self.thread_table.get(&thread_id).expect("thread not found");
                        entry.task_tx.send((task_id, task)).await?;
                    }
                    None => {
                        self.thread_ready_buf.push(thread_id);
                    }
                }
                Ok(())
            }

            TaskOutputKind::TaskComplete((task_id, task_result)) => {
                self.store.push_output(task_id, task_result).await?;
                Ok(())
            }
        }
    }

    async fn on_terminate(&mut self) -> Result<(), anyhow::Error> {
        let thread_ids = self.thread_table.keys().cloned().collect::<Vec<_>>();

        for thread_id in thread_ids {
            if let Some(entry) = self.thread_table.remove(&thread_id) {
                {
                    entry.task_tx;
                } // Drop senders

                debug!(thread_id; "Joining thread");
                let result = with_gvl!({
                    entry
                        .thread
                        .funcall_public::<_, _, magnus::Value>("join", (30,))
                });

                if let Err(e) = result {
                    match e.error_type() {
                        ErrorType::Jump(tag) => {
                            debug!(thread_id, tag:?; "Jump error when joining thread")
                        }
                        ErrorType::Exception(exception) => {
                            warn!(thread_id, exception:?; "Exception when joining thread")
                        }
                        ErrorType::Error(error, stacktrace) => {
                            error!(thread_id, error:?; "Could not join thread: {stacktrace:?}")
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
