//!
//! A trivial, inefficient implementation of storing tasks.
//!
//! Stores all task IDs, values, and results in memory. This means all data will be lost if the application crashes.
//! This implementation is meant to be a starting point to decouple the task execution logic from the logic of managing
//! task queues and results.
//!

use crate::events::{Task, TaskId, TaskResult};
use crate::mem::TaskMemory;
use std::collections::HashMap;
use std::future::Future;
use std::ops::DerefMut;
use std::pin::{pin, Pin};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::SystemTime;
use tokio::sync::Mutex;

struct GetCompleted {
    task_id: TaskId,
    table: Arc<Mutex<HashMap<TaskId, TaskTableEntry>>>,
    abort: Arc<AtomicBool>
}

impl GetCompleted {
    pub fn new(task_id: TaskId, tasks_in_memory: &TasksInMemory) -> Self {
        Self {
            task_id,
            table: tasks_in_memory.table.clone(),
            abort: tasks_in_memory.abort_futures.clone()
        }
    }
}

impl Future for GetCompleted {
    type Output = Result<Option<(TaskId, TaskResult)>, anyhow::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.abort.load(Ordering::Acquire) {
            return Poll::Ready(Ok(None))
        }
        
        let table = pin!(self.table.lock());
        match table.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(mut table) => {
                match table.remove_entry(&self.task_id) {
                    Some((task_id, TaskTableEntry::Complete(result))) => {
                        Poll::Ready(Ok(Some((task_id, result))))
                    },
                    Some((task_id, entry)) => {
                        table.insert(task_id, entry);
                        Poll::Pending
                    },
                    None => {
                        Poll::Ready(Ok(None))
                    }
                }
            }
        }
    }
}

enum TaskTableEntry {
    Enqueued(Task),
    Busy,
    Complete(TaskResult)
}

pub struct TasksInMemory {
    table: Arc<Mutex<HashMap<TaskId, TaskTableEntry>>>,
    next_task_queue: Mutex<Vec<TaskId>>,
    abort_futures: Arc<AtomicBool>
}

impl TasksInMemory {
    pub fn new() -> Self {
        Self {
            table: Arc::new(Mutex::new(HashMap::new())),
            next_task_queue: Mutex::new(Vec::new()),
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

impl TaskMemory for TasksInMemory {
    async fn push_queue(&self, value: Task) -> Result<TaskId, anyhow::Error> {
        let task_id = Self::next_id();
        
        let mut next_task_queue = self.next_task_queue.lock().await;
        let mut table = self.table.lock().await;

        table.insert(task_id, TaskTableEntry::Enqueued(value));
        next_task_queue.push(task_id);

        Ok(task_id)
    }

    async fn pop_queue(&self) -> Result<Option<(TaskId, Task)>, anyhow::Error> {
        let mut next_task_queue = self.next_task_queue.lock().await;
        match next_task_queue.pop() {
            Some(task_id) => {
                let mut table = self.table.lock().await;
                match table.remove_entry(&task_id) {
                    // Only remove enqueued tasks
                    Some((task_id, TaskTableEntry::Enqueued(task))) => {
                        table.insert(task_id, TaskTableEntry::Busy);
                        Ok(Some((task_id, task)))
                    },

                    Some((task_id, other)) => {
                        table.insert(task_id, other);
                        Ok(None)
                    }

                    _ => Ok(None)
                }
            },

            None => Ok(None)
        }
    }

    async fn insert_completed(&self, task_id: TaskId, value: TaskResult) -> Result<TaskId, anyhow::Error> {
        let mut table = self.table.lock().await;
        table.insert(task_id, TaskTableEntry::Complete(value));
        Ok(task_id)
    }

    async fn remove_completed(&self, task_id: TaskId) -> Result<Option<(TaskId, TaskResult)>, anyhow::Error> {
        GetCompleted::new(task_id, &self).await
    }
}
