use crate::executor::thread_main::rb_thread_main;
use crate::mavrik::MavrikOptions;
use crate::messaging::{Task, TaskId, TaskResult};
use crate::rb::in_ruby;
use crate::service::MavrikService;
use crate::store::ProcessStore;
use log::{debug, error, trace};
use magnus::value::ReprValue;
use std::collections::HashMap;
use tokio::select;
use tokio::sync::mpsc;

/// ID associated with a Ruby thread created by the task executor.
pub type ThreadId = usize;

#[derive(Debug)]
pub enum ThreadMessage {
    ThreadReady(ThreadId),
    TaskComplete((TaskId, TaskResult))
}

#[derive(Debug)]
pub enum TaskOutputKind {
    ThreadReady(ThreadId),
    TaskComplete((TaskId, TaskResult)),
    NextTask((TaskId, Task))
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


pub struct TaskExecutor<Store> {
    store: Store,
    thread_table: HashMap<ThreadId, ThreadTableEntry>,
    messages_rx: mpsc::Receiver<ThreadMessage>,
    task_buf: Vec<(TaskId, Task)>,
    thread_ready_buf: Vec<ThreadId>,
}

impl<Store> TaskExecutor<Store> {
    /// Create a new task executor service.
    ///
    /// # Arguments
    ///
    /// `options` - Options for configuring the task executor.
    /// `params` - Values for constructing the task executor.
    ///
    pub fn new(options: &MavrikOptions, store: Store) -> Result<Self, anyhow::Error> {
        let rb_thread_count = options.get("rb_thread_count", 4usize)?;

        let mut thread_table = HashMap::new();
        let (messages_tx, messages_rx) = mpsc::channel(rb_thread_count);
        let task_buf = Vec::new();
        let thread_ready_buf = Vec::new();
        
        in_ruby(|r| {
            for thread_id in 0..rb_thread_count {
                let (task_tx, task_rx) = mpsc::channel(1);
                let messages_tx = messages_tx.clone();
                let thread = r.thread_create_from_fn(move |r| rb_thread_main(r, thread_id, messages_tx, task_rx));

                thread_table.insert(thread_id, ThreadTableEntry { thread, task_tx });
            }
        });

        Ok(Self { store, thread_table, messages_rx, task_buf, thread_ready_buf })
    }
}

impl<Store> MavrikService for TaskExecutor<Store>
where
    Store: ProcessStore<Id = TaskId, Error = anyhow::Error>
{
    type TaskOutput = Result<TaskOutputKind, anyhow::Error>;

    async fn poll_task(&mut self) -> Self::TaskOutput {
        select! { 
            result = self.store.next(), if self.task_buf.len() < 100 => {
                Ok(TaskOutputKind::NextTask(result?))
            },
            
            Some(message) = self.messages_rx.recv() => {
                Ok(message.into())
            }
        }
    }

    async fn on_task_ready(&mut self, output: Self::TaskOutput) -> Result<(), anyhow::Error> {
        match output? {
            TaskOutputKind::NextTask((task_id, task)) => {
                match self.thread_ready_buf.pop() {
                    Some(thread_id) => {
                        let entry = self.thread_table.get(&thread_id).expect("thread not found");
                        entry.task_tx.send((task_id, task)).await?;
                    },
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
                    },
                    None => {
                        self.thread_ready_buf.push(thread_id);
                    }
                }
                Ok(())
            },
            
            TaskOutputKind::TaskComplete((task_id, task_result)) => {
                self.store.publish(task_id, task_result).await?;
                Ok(())
            }
        }
    }

    async fn on_terminate(&mut self) -> Result<(), anyhow::Error> {
        let thread_ids = self.thread_table
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        
        for thread_id in thread_ids {
            if let Some(entry) = self.thread_table.remove(&thread_id) {
                { entry.task_tx; } // Drop senders
                
                debug!("Joining thread");
                let result = entry.thread.funcall_public::<_, _, magnus::Value>("join", (30,));
                if let Err(e) = result {
                    error!(e:?; "Could not join thread");
                }
            }
        }

        Ok(())
    }
}
