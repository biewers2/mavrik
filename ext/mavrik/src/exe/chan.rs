use tokio::sync::mpsc;
use crate::events::{Task, TaskId};
use crate::exe::ThreadId;

#[derive(Debug)]
pub enum ThreadMessage {
    ThreadReady(ThreadId),
    Awaited {
        task_id: TaskId,
        value: String
    }
}

pub struct ExecutorChannel {
    task_rx: mpsc::Receiver<Task>,
    thread_tx: mpsc::Sender<ThreadMessage>
}

impl ExecutorChannel {
    pub fn new(task_rx: mpsc::Receiver<Task>, thread_tx: mpsc::Sender<ThreadMessage>) -> Self {
        Self { task_rx, thread_tx }
    }
    
    pub async fn next_task(&mut self) -> Option<Task> {
        self.task_rx.recv().await
    }

    pub async fn thread_ready(&self, thread_id: ThreadId) -> Result<(), anyhow::Error> {
        let message = ThreadMessage::ThreadReady(thread_id);
        self.thread_tx.send(message).await?;
        Ok(())
    }

    pub async fn task_awaited(&self, task_id: TaskId, value: String) -> Result<(), anyhow::Error> {
        let message = ThreadMessage::Awaited {
            task_id,
            value
        };
        self.thread_tx.send(message).await?;
        Ok(())
    }
}
