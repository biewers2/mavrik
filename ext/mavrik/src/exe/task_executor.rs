use crate::events::{AwaitedTask, ExeEvent, MavrikEvent, Task, TaskId, TaskResult};
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
use crate::service::Service;

/// ID associated with a Ruby thread created by the task executor.
pub type ThreadId = usize;

/// Options for configuring the task executor.
pub struct TaskExecutorOptions {
    // Number of Ruby threads to create.
    pub rb_thread_count: usize,
}

/// Parameters for creating the task executor.
pub struct TaskExecutorParams {
    /// Where to send events to.
    pub event_tx: mpsc::Sender<MavrikEvent>
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
    task_tx: mpsc::Sender<Task>
}

/// A future that waits for a task to be completed by a Ruby thread.
struct AwaitTaskFuture {
    /// ID of the task.
    task_id: TaskId,

    /// Reference to the task result buffer.
    /// This is where the future looks for the task ID when polled.
    buf: Arc<Mutex<HashMap<TaskId, TaskResult>>>
}

impl AwaitTaskFuture {
    /// Create a new future to await for the completion of a task.
    ///
    /// # Arguments
    ///
    /// `task_id` - ID of the task to wait for.
    /// `buf` - Reference to the table storing the task results.
    ///
    pub fn new(task_id: TaskId, results_table: Arc<Mutex<HashMap<TaskId, TaskResult>>>) -> Self {
        Self { task_id, buf: results_table }
    }
}

impl Future for AwaitTaskFuture {
    type Output = AwaitedTask;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let buf = pin!(self.buf.lock());
        match buf.poll(cx) {
            // Continue pending if lock isn't ready.
            Poll::Pending => Poll::Pending,

            // When ready, check if the task ID is present in the results table by removing it.
            Poll::Ready(mut buf) => match buf.remove(&self.task_id) {
                Some(result) => Poll::Ready(AwaitedTask {
                    id: self.task_id.clone(),
                    result
                }),
                None => Poll::Pending
            }
        }
    }
}

/// Task executor service, used to execute tasks concurrently in Ruby threads.
pub struct TaskExecutor {
    /// Table mapping thread IDs to threads and associated structs.
    thread_table: HashMap<ThreadId, ThreadTableEntry>,

    /// Table mapping task IDs to their results.
    results_table: Arc<Mutex<HashMap<TaskId, TaskResult>>>,

    /// Where to receive messages from threads.
    thread_rx: mpsc::Receiver<ThreadMessage>,

    /// Enqueued tasks to be picked up by threads as they become available.
    task_buf: Vec<Task>,

    /// List of threads available to process tasks listed by their IDs.
    ready_buf: Vec<ThreadId>
}

impl TaskExecutor {
    /// Create a new task executor service.
    ///
    /// # Arguments
    ///
    /// `options` - Options for configuring the task executor.
    /// `params` - Values for constructing the task executor.
    ///
    pub fn new(options: TaskExecutorOptions, params: TaskExecutorParams) -> Result<Self, anyhow::Error> {
        let TaskExecutorOptions { rb_thread_count, .. } = options;
        let TaskExecutorParams { .. } = params;

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
            thread_table,
            thread_rx,
            task_buf: vec![],
            results_table: Arc::new(Mutex::new(HashMap::new())),
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
        AwaitTaskFuture::new(task_id, self.results_table.clone()).await
    }

    async fn run_task_on_thread(&self, thread_id: ThreadId, task: Task) -> Result<(), anyhow::Error> {
        trace!(thread_id, task:?; "Running on thread");

        let entry = self.thread_table.get(&thread_id).ok_or(anyhow!("thread with ID {thread_id} not found"))?;
        entry.task_tx.send(task).await?;

        Ok(())
    }
}

impl Service for TaskExecutor {
    type TaskOutput = Option<ThreadMessage>;
    type Message = ExeEvent;

    async fn poll_task(&mut self) -> Self::TaskOutput {
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

            Some(ThreadMessage::Awaited { task_id, task_result }) => {
                trace!(task_id:?, task_result:?; "Task awaited");
                self.results_table.lock().await.insert(task_id, task_result);
                Ok(())
            },

            None => Err(anyhow!("Thread messaging channel unexpectedly closed early"))
        }
    }

    async fn on_message(&mut self, message: Self::Message) -> Result<(), anyhow::Error> {
        match message {
            ExeEvent::NewTask(task) => {
                self.execute(task).await?
            },

            ExeEvent::AwaitTask {
                task_id,
                value_tx
            } => {
                let awaited_task = self.get_task_result(task_id).await;
                let result = value_tx.send(awaited_task);

                // Put the value back in the buffer if it failed to send.
                if let Err(AwaitedTask { id, result }) = result {
                    self.results_table.lock().await.insert(id, result);
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
                if let Err(e) = entry.thread.funcall_public::<_, _, magnus::Value>("join", (30,)) {
                    error!(thread_id, e:?; "Could not join thread");
                }
            }
        }

        Ok(())
    }
}
