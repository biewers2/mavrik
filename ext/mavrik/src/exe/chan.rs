use anyhow::Context;
use tokio::sync::mpsc;
use crate::events::{Task, TaskId, TaskResult};
use crate::exe::ThreadId;

#[derive(Debug, Eq, PartialEq)]
pub enum ThreadMessage {
    ThreadReady(ThreadId),
    Completed {
        task_id: TaskId,
        task_result: TaskResult
    }
}

pub struct ExecutorChannel {
    task_rx: mpsc::Receiver<(TaskId, Task)>,
    thread_tx: mpsc::Sender<ThreadMessage>
}

impl ExecutorChannel {
    pub fn new(task_rx: mpsc::Receiver<(TaskId, Task)>, thread_tx: mpsc::Sender<ThreadMessage>) -> Self {
        Self { task_rx, thread_tx }
    }
    
    pub async fn next_task(&mut self) -> Option<(TaskId, Task)> {
        self.task_rx.recv().await
    }

    pub async fn thread_ready(&self, thread_id: ThreadId) -> Result<(), anyhow::Error> {
        let message = ThreadMessage::ThreadReady(thread_id);
        self.thread_tx.send(message).await.context("sending ready thread from thread")?;
        Ok(())
    }

    pub async fn task_complete(&self, task_id: TaskId, task_result: TaskResult) -> Result<(), anyhow::Error> {
        let message = ThreadMessage::Completed {
            task_id,
            task_result
        };
        self.thread_tx.send(message).await.context("sending awaited from thread")?;
        Ok(())
    }
}
