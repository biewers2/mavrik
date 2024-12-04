use crate::executor::{ThreadId, ThreadMessage};
use crate::messaging::{Task, TaskId, TaskResult};
use crate::rb::util::{mavrik_error, module_mavrik, MRHash};
use crate::runtime::async_runtime;
use crate::{with_gvl, without_gvl};
use anyhow::anyhow;
use log::{error, trace};
use magnus::value::ReprValue;
use magnus::{Class, Module, RClass, RHash, Ruby};
use tokio::sync::mpsc;

pub fn rb_thread_main(
    _ruby: &Ruby,
    thread_id: ThreadId,
    mut messages_tx: mpsc::Sender<ThreadMessage>,
    mut task_rx: mpsc::Receiver<(TaskId, Task)>,
) -> Result<(), magnus::Error> {
    let execute_task = module_mavrik()
        .const_get::<_, RClass>("ExecuteTask")?
        .new_instance(())?;

    let result = without_gvl!({
        async_runtime().block_on(thread_loop(thread_id, execute_task, &mut messages_tx, &mut task_rx))
    });

    if let Err(e) = &result {
        error!(e:?; "Failure in Ruby thread main");
    }

    result.map_err(mavrik_error)
}

async fn thread_loop(
    thread_id: ThreadId,
    execute_task: magnus::Value,
    messages_tx: &mut mpsc::Sender<ThreadMessage>,
    task_rx: &mut mpsc::Receiver<(TaskId, Task)>,
) -> Result<(), anyhow::Error> {
    // Notify executor this thread is ready
    messages_tx.send(ThreadMessage::ThreadReady(thread_id)).await?;
    
    while let Some((task_id, task)) = task_rx.recv().await {
        let Task { definition, args, kwargs, .. } = &task;
        trace!(definition, args, kwargs; "Task executing");

        // Any errors raised in the task will be captured in `TaskResult`
        // We return nested results so we can provide context if things fail.
        let result: Result<Result<TaskResult, magnus::Error>, magnus::Error> = with_gvl!({
            let ctx = MRHash::new();
            ctx.set_sym("definition", definition.as_str())?;
            ctx.set_sym("args", args.as_str())?;
            ctx.set_sym("kwargs", kwargs.as_str())?;

            execute_task
                .funcall_public::<_, (RHash,), RHash>("call", (ctx.0,))
                .map(|h| h.try_into())
        });
        
        let task_result = result
            .map_err(|e| anyhow!("task execution failed: {e}"))?
            .map_err(|e| anyhow!("converting returned hash to task result failed: {e}"))?;

        trace!(definition, args, kwargs; "Task complete");
        messages_tx.send(ThreadMessage::TaskComplete((task_id, task_result))).await?;
    }

    Ok(())
}
