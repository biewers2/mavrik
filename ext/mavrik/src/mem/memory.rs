use crate::events::{Task, TaskId, TaskResult};

pub trait TaskMemory {
    async fn push_queue(&self, value: Task) -> Result<TaskId, anyhow::Error>;

    async fn pop_queue(&self) -> Result<Option<(TaskId, Task)>, anyhow::Error>;
    
    async fn insert_completed(&self, id: TaskId, value: TaskResult) -> Result<TaskId, anyhow::Error>;

    async fn remove_completed(&self, id: TaskId) -> Result<Option<(TaskId, TaskResult)>, anyhow::Error>;
}