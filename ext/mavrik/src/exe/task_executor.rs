use crate::events::{AwaitedTask, ExeEvent, MavrikEvent, Task, TaskId};
use crate::rb::in_ruby;
use log::{debug, error, trace};
use std::collections::HashMap;
use std::future::Future;
use std::pin::{pin, Pin};
use std::sync::Arc;
use std::task::{Context, Poll};
use anyhow::anyhow;
use magnus::value::ReprValue;
use tokio::sync::{mpsc, Mutex};
use crate::exe::chan::{ExecutorChannel, ThreadMessage};
use crate::exe::thread_main::rb_thread_main;
use crate::with_gvl;
use crate::service::Service;

pub type ThreadId = usize;

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

struct AwaitTaskFuture {
    task_id: TaskId,
    buf: Arc<Mutex<HashMap<TaskId, String>>>
}

impl AwaitTaskFuture {
    pub fn new(task_id: TaskId, buf: Arc<Mutex<HashMap<TaskId, String>>>) -> Self {
        Self { task_id, buf }
    }
}

impl Future for AwaitTaskFuture {
    type Output = AwaitedTask;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let buf = pin!(self.buf.lock());
        match buf.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(mut buf) => match buf.remove(&self.task_id) {
                Some(value) => Poll::Ready(AwaitedTask {
                    id: self.task_id.clone(),
                    value
                }),
                None => Poll::Pending
            }
        }
    }
}

pub struct TaskExecutor {
    thread_table: HashMap<usize, ThreadTableEntry>,
    thread_rx: mpsc::Receiver<ThreadMessage>,
    task_buf: Vec<Task>,
    awaited_buf: Arc<Mutex<HashMap<TaskId, String>>>,
    ready_buf: Vec<ThreadId>
}

impl TaskExecutor {
    pub fn new(options: TaskExecutorOptions, params: TaskExecutorParams) -> Result<Self, anyhow::Error> {
        let TaskExecutorOptions { rb_thread_count } = options;
        let TaskExecutorParams { .. } = params;

        let (thread_tx, thread_rx) = mpsc::channel::<ThreadMessage>(100);

        let thread_table = with_gvl!({
            in_ruby::<Result<_, anyhow::Error>>(|ruby| {
                let mut table = HashMap::new();
                for i in 0..rb_thread_count {
                    let thread_id = i;

                    let (task_tx, task_rx) = mpsc::channel::<Task>(1000);
                    let chan = ExecutorChannel::new(task_rx, thread_tx.clone());

                    let thread = ruby.thread_create_from_fn(move |r| rb_thread_main(r, thread_id, chan));
                    table.insert(thread_id, ThreadTableEntry { thread, task_tx });
                }
        
                Ok(table)
            })
        })?;

        Ok(Self {
            thread_table,
            thread_rx,
            task_buf: vec![],
            awaited_buf: Arc::new(Mutex::new(HashMap::new())),
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

    pub async fn get_task_result(&self, task_id: TaskId) -> AwaitedTask {
        AwaitTaskFuture::new(task_id, self.awaited_buf.clone()).await
    }

    async fn run_task_on_thread(&self, thread_id: ThreadId, task: Task) -> Result<(), anyhow::Error> {
        trace!("Running {task:?} on thread {thread_id}");

        let entry = self.thread_table.get(&thread_id).ok_or(anyhow!("thread with ID {thread_id} not found"))?;
        entry.task_tx.send(task).await?;

        Ok(())
    }
}

impl Service for TaskExecutor {
    type TaskOutput = Option<ThreadMessage>;
    type Message = ExeEvent;

    async fn call_task(&mut self) -> Self::TaskOutput {
        self.thread_rx.recv().await
    }

    async fn on_task_ready(&mut self, message: Self::TaskOutput) -> Result<(), anyhow::Error> {
        match message {
            Some(ThreadMessage::ThreadReady(thread_id)) => {
                match self.task_buf.pop() {
                    Some(task) => self.run_task_on_thread(thread_id, task).await?,
                    None => {
                        trace!("No tasks in queue, pushing thread on to queue");
                        self.ready_buf.push(thread_id);
                    }
                }
                Ok(())
            },

            Some(ThreadMessage::Awaited { task_id, value }) => {
                trace!("Task {task_id} awaited producing {value}");
                self.awaited_buf.lock().await.insert(task_id, value);
                Ok(())
            },

            None => Err(anyhow!("Thread messaging channel unexpectedly closed early"))
        }
    }

    async fn on_message(&mut self, message: Self::Message) -> Result<(), anyhow::Error> {
        match message {
            ExeEvent::NewTask(task) => self.execute(task).await?,
            ExeEvent::AwaitTask {
                task_id,
                value_tx
            } => {
                let awaited_task = self.get_task_result(task_id).await;
                let result = value_tx.send(awaited_task);

                // Put the value back in the buffer if it failed to send.
                if let Err(AwaitedTask { id, value }) = result {
                    self.awaited_buf.lock().await.insert(id, value);
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

                debug!("Joining thread {thread_id}");
                if let Err(e) = entry.thread.funcall_public::<_, _, magnus::Value>("join", (30,)) {
                    error!("Could not join thread {thread_id}: {e}");
                }
            }
        }

        Ok(())
    }
}
