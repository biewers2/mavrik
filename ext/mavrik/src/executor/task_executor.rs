use crate::mavrik::MavrikOptions;
use crate::executor::thread_main::rb_thread_main;
use crate::rb::in_ruby;
use crate::service::MavrikService;
use crate::store::ProcessStore;
use log::{debug, error};
use magnus::value::ReprValue;
use tokio::sync::mpsc;
use crate::messaging::TaskId;

/// ID associated with a Ruby thread created by the task executor.
pub type ThreadId = usize;

pub struct TaskExecutor {
    threads: Vec<magnus::Thread>,
    term_txs: Vec<mpsc::Sender<()>>,
}

impl TaskExecutor {
    /// Create a new task executor service.
    ///
    /// # Arguments
    ///
    /// `options` - Options for configuring the task executor.
    /// `params` - Values for constructing the task executor.
    ///
    pub fn new<Store>(options: &MavrikOptions, store: Store) -> Result<Self, anyhow::Error>
    where
        Store: ProcessStore<Id = TaskId, Error = anyhow::Error> + Clone + Send + Sync + 'static,
    {
        let rb_thread_count = options.get("rb_thread_count", 4usize)?;

        let mut threads = vec![];
        let mut term_txs = vec![];

        in_ruby(|r| {
            for _ in 0..rb_thread_count {
                let (term_tx, term_rx) = mpsc::channel(1);
                let store = store.clone();
                let thread = r.thread_create_from_fn(move |r| rb_thread_main(r, store, term_rx));

                term_txs.push(term_tx);
                threads.push(thread);
            }
        });

        Ok(Self { threads, term_txs })
    }
}

impl MavrikService for TaskExecutor {
    type TaskOutput = ();
    type Message = ();

    async fn on_terminate(&mut self) -> Result<(), anyhow::Error> {
        while let Some(term_tx) = self.term_txs.pop() {
            term_tx.send(()).await?;
        }
        
        while let Some(thread) = self.threads.pop() {
            debug!("Joining thread");
            let result: Result<magnus::Value, magnus::Error> = thread.funcall_public("join", (30,));
            if let Err(e) = result {
                error!(e:?; "Could not join thread");
            }
        }

        Ok(())
    }
}
