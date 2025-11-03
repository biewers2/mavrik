//!
//! A trivial, inefficient implementation of storing tasks.
//!
//! Stores all task IDs, values, and results in memory. This means all data will be lost if the
//! application crashes. This implementation is meant to be a starting point to decouple the task
//! execution logic from the logic of managing task queues and results.
//!

use crate::messaging::{Task, TaskId};
use crate::store::store_state::{StoreState, StoredTask, StoredTaskStatus};
use crate::store::{ProcessStore, PullStore, PushStore, QueryStore};
use log::trace;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::future::Future;
use std::ops::DerefMut;
use std::pin::{pin, Pin};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::SystemTime;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct TasksInMemory {
    queue_wakers: Arc<Mutex<Vec<Waker>>>,
    queue: Arc<Mutex<Vec<(TaskId, String)>>>,
    busy: Arc<Mutex<HashMap<TaskId, String>>>,
    completed_wakers: Arc<Mutex<HashMap<TaskId, Waker>>>,
    completed: Arc<Mutex<HashMap<TaskId, String>>>,
}

impl TasksInMemory {
    pub fn new() -> Self {
        Self {
            queue_wakers: Arc::new(Mutex::new(Vec::new())),
            queue: Arc::new(Mutex::new(Vec::new())),
            busy: Arc::new(Mutex::new(HashMap::new())),
            completed_wakers: Arc::new(Mutex::new(HashMap::new())),
            completed: Arc::new(Mutex::new(HashMap::new())),
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

    async fn push<S, V>(&self, queue: S, value: V) -> Result<Self::Id, Self::Error>
    where
        S: AsRef<str> + Send,
        V: Serialize + Send,
    {
        let value = serde_json::to_string(&value)?;
        let id = Self::next_id();

        let mut queue = self.queue.lock().await;
        queue.push((id, value));
        let mut wakers = self.queue_wakers.lock().await;
        if let Some(waker) = wakers.pop() {
            waker.wake();
        }

        Ok(id)
    }
}

impl PullStore for TasksInMemory {
    type Id = TaskId;
    type Error = anyhow::Error;

    async fn pull<D>(&self, id: Self::Id) -> Result<D, Self::Error>
    where
        D: DeserializeOwned,
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

    async fn dequeue<D>(&self) -> Result<(Self::Id, D), Self::Error>
    where
        D: DeserializeOwned,
    {
        let (id, value) = NextTask::new(&self).await?;
        trace!(id, value:?; "Pulling next task for processing");

        let value = serde_json::from_str(&value)?;
        Ok((id, value))
    }

    async fn publish_result<S>(&self, id: Self::Id, output: S) -> Result<(), Self::Error>
    where
        S: Serialize + Send,
    {
        let output = serde_json::to_string(&output)?;
        trace!(id, output:?; "Publishing completed task");

        let mut completed = self.completed.lock().await;
        completed.insert(id, output);
        let mut wakers = self.completed_wakers.lock().await;
        if let Some(waker) = wakers.remove(&id) {
            waker.wake();
        }

        Ok(())
    }
}

struct PullTask {
    task_id: TaskId,
    completed_wakers: Arc<Mutex<HashMap<TaskId, Waker>>>,
    completed: Arc<Mutex<HashMap<TaskId, String>>>,
}

impl PullTask {
    pub fn new(task_id: TaskId, tasks_in_memory: &TasksInMemory) -> Self {
        Self {
            task_id,
            completed_wakers: tasks_in_memory.completed_wakers.clone(),
            completed: tasks_in_memory.completed.clone(),
        }
    }
}

impl Future for PullTask {
    type Output = Result<String, anyhow::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let completed = pin!(self.completed.lock());
        match completed.poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(mut completed) => {
                if let Some((_, task)) = completed.remove_entry(&self.task_id) {
                    return Poll::Ready(Ok(task));
                }
            }
        };

        let wakers = pin!(self.completed_wakers.lock());
        match wakers.poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(mut wakers) => {
                if let Some(waker) = wakers.get_mut(&self.task_id) {
                    waker.clone_from(cx.waker());
                } else {
                    wakers.insert(self.task_id, cx.waker().clone());
                };
            }
        };

        Poll::Pending
    }
}

struct NextTask {
    queue_wakers: Arc<Mutex<Vec<Waker>>>,
    queue: Arc<Mutex<Vec<(TaskId, String)>>>,
}

impl NextTask {
    pub fn new(tasks_in_memory: &TasksInMemory) -> Self {
        Self {
            queue_wakers: tasks_in_memory.queue_wakers.clone(),
            queue: tasks_in_memory.queue.clone(),
        }
    }
}

impl Future for NextTask {
    type Output = Result<(TaskId, String), anyhow::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let queue = pin!(self.queue.lock());
        match queue.poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(mut completed) => {
                if let Some(task) = completed.pop() {
                    return Poll::Ready(Ok(task));
                }
            }
        };

        let wakers = pin!(self.queue_wakers.lock());
        match wakers.poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(mut wakers) => {
                wakers.push(cx.waker().clone());
            }
        };

        Poll::Pending
    }
}

impl QueryStore for TasksInMemory {
    type Error = anyhow::Error;

    async fn state(&self) -> Result<StoreState, Self::Error> {
        let mut tasks = vec![];
        for (task_id, task_str) in self.queue.lock().await.iter() {
            let task: Task = serde_json::from_str(task_str)?;
            tasks.push(StoredTask {
                id: task_id.clone(),
                status: StoredTaskStatus::Enqueued,
                definition: task.definition,
                args: task.args,
                kwargs: task.kwargs,
            });
        }

        for (task_id, task_str) in self.busy.lock().await.iter() {
            let task: Task = serde_json::from_str(task_str)?;
            tasks.push(StoredTask {
                id: task_id.clone(),
                status: StoredTaskStatus::Processing,
                definition: task.definition,
                args: task.args,
                kwargs: task.kwargs,
            });
        }

        for (task_id, task_str) in self.completed.lock().await.iter() {
            let task: Task = serde_json::from_str(task_str)?;
            tasks.push(StoredTask {
                id: task_id.clone(),
                status: StoredTaskStatus::Completed,
                definition: task.definition,
                args: task.args,
                kwargs: task.kwargs,
            });
        }

        Ok(StoreState { tasks })
    }
}
