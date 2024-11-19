//!
//! A trivial, inefficient implementation of storing tasks.
//!
//! Stores all task IDs, values, and results in memory. This means all data will be lost if the application crashes.
//! This implementation is meant to be a starting point to decouple the task execution logic from the logic of managing
//! task queues and results.
//!

use crate::messaging::TaskId;
use crate::store::{ProcessStore, PullStore, PushStore};
use anyhow::anyhow;
use log::trace;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::future::Future;
use std::ops::DerefMut;
use std::pin::{pin, Pin};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::SystemTime;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
enum TaskTableEntry {
    Enqueued(String),
    Busy,
    Complete(String)
}

#[derive(Debug, Clone)]
pub struct TasksInMemory {
    table: Arc<Mutex<HashMap<TaskId, TaskTableEntry>>>,
    abort_futures: Arc<AtomicBool>
}

impl TasksInMemory {
    pub fn new() -> Self {
        Self {
            table: Arc::new(Mutex::new(HashMap::new())),
            abort_futures: Arc::new(AtomicBool::new(false))
        }
    }

    fn next_id() -> TaskId {
        // (timestamp, counter)
        static LAST: std::sync::Mutex<(u128, u32)> = std::sync::Mutex::new((0, 0));

        let mut guard = LAST.lock().unwrap();
        let (last_timestamp, last_count) = guard.deref_mut();

        // Use system timestamp as primary identifier
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        // Append counter to end in case of conflict with timestamp.
        if *last_timestamp == timestamp {
            *last_count += 1;
        } else {
            *last_timestamp = timestamp;
            *last_count = 0;
        };
        let n = *last_count;

        TaskId::from_parts(timestamp, n)
    }
}

impl PushStore for TasksInMemory {
    type Id = TaskId;
    type Error = anyhow::Error;

    async fn push<S>(&self, value: S) -> Result<Self::Id, Self::Error>
    where
        S: Serialize
    {
        let value = serde_json::to_string(&value)?;
        let id = Self::next_id();
        
        let mut table = self.table.lock().await;
        
        trace!(id, value:?; "Pushing on to store");
        table.insert(id, TaskTableEntry::Enqueued(value));

        Ok(id)
    }
}

impl PullStore for TasksInMemory {
    type Id = TaskId;
    type Error = anyhow::Error;

    async fn pull<D>(&self, id: Self::Id) -> Result<D, Self::Error>
    where
        D: DeserializeOwned
    {
        let output = PullTask::new(id, &self).await?;
        trace!(id, output:?; "Pulled from store");
        
        let output = serde_json::from_str(&output)?;
        Ok(output)
    }
}

impl ProcessStore for TasksInMemory {
    type Id = TaskId;
    type Error = anyhow::Error;

    async fn next<D>(&self) -> Result<(Self::Id, D), Self::Error>
    where
        D: DeserializeOwned
    {
        let (id, value) = NextTask::new(&self).await?;
        trace!(id, value:?; "Pulling next task for processing");
        
        let value = serde_json::from_str(&value)?;
        Ok((id, value))
    }

    async fn publish<S>(&self, id: Self::Id, output: S) -> Result<(), Self::Error>
    where
        S: Serialize
    {
        let output = serde_json::to_string(&output)?;
        trace!(id, output:?; "Publishing completed task");
        
        let mut table = self.table.lock().await;
        table.insert(id, TaskTableEntry::Complete(output));
        
        Ok(())
    }
}

struct PullTask {
    task_id: TaskId,
    table: Arc<Mutex<HashMap<TaskId, TaskTableEntry>>>,
    abort: Arc<AtomicBool>
}

impl PullTask {
    pub fn new(task_id: TaskId, tasks_in_memory: &TasksInMemory) -> Self {
        Self {
            task_id,
            table: tasks_in_memory.table.clone(),
            abort: tasks_in_memory.abort_futures.clone()
        }
    }
}

impl Future for PullTask {
    type Output = Result<String, anyhow::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.abort.load(Ordering::Acquire) {
            return Poll::Ready(Err(anyhow!("aborted")))
        }

        let table = pin!(self.table.lock());
        match table.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(mut table) => {
                match table.remove_entry(&self.task_id) {
                    Some((_, TaskTableEntry::Complete(output))) => {
                        Poll::Ready(Ok(output))
                    },
                    Some((task_id, entry)) => {
                        table.insert(task_id, entry);
                        Poll::Pending
                    },
                    None => {
                        Poll::Pending
                    }
                }
            }
        }
    }
}

struct NextTask {
    table: Arc<Mutex<HashMap<TaskId, TaskTableEntry>>>,
    abort: Arc<AtomicBool>
}

impl NextTask {
    pub fn new(tasks_in_memory: &TasksInMemory) -> Self {
        Self {
            table: tasks_in_memory.table.clone(),
            abort: tasks_in_memory.abort_futures.clone()
        }
    }
}

impl Future for NextTask {
    type Output = Result<(TaskId, String), anyhow::Error>;
    
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if self.abort.load(Ordering::Acquire) {
            return Poll::Ready(Err(anyhow!("aborted")))
        }
        
        let table = pin!(self.table.lock());
        match table.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(mut table) => {
                let id = match table.keys().next() {
                    Some(id) => id.clone(),
                    None => return Poll::Pending
                };
                
                match table.remove_entry(&id) {
                    Some((task_id, TaskTableEntry::Enqueued(task))) => {
                        table.insert(task_id, TaskTableEntry::Busy);
                        Poll::Ready(Ok((task_id, task)))
                    },
                    Some((task_id, entry)) => {
                        table.insert(task_id, entry);
                        Poll::Pending
                    },
                    None => {
                        Poll::Pending
                    }
                }       
            }
        }
    }
}
