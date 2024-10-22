use anyhow::anyhow;
use log::{trace};
use magnus::{kwargs, Class, Module, RClass, Ruby};
use magnus::value::ReprValue;
use crate::events::Task;
use crate::exe::ThreadId;
use crate::rb::{mavrik_error, module_mavrik};
use crate::runtime::async_runtime;
use crate::{with_gvl, without_gvl};
use crate::exe::chan::ExecutorChannel;

pub fn rb_thread_main(_ruby: &Ruby, thread_id: ThreadId, mut exe: ExecutorChannel) -> Result<(), magnus::Error> {
    let execute_task = module_mavrik()
        .const_get::<_, RClass>("ExecuteTask")?
        .new_instance(())?;

    let result = without_gvl!({
        async_runtime().block_on(thread_loop(thread_id, execute_task, &mut exe))
    });

    result.map_err(mavrik_error)
}

async fn thread_loop(thread_id: ThreadId, execute_task: magnus::Value, exe: &mut ExecutorChannel) -> Result<(), anyhow::Error> {
    // Mark thread ready at the start.
    exe.thread_ready(thread_id).await?;

    while let Some(task) = exe.next_task().await {
        let Task { id, definition, input_args, input_kwargs, .. } = &task;
        trace!("({id}) Executing '{definition}' with args '{input_args}' and kwargs '{input_kwargs}'");

        let result = with_gvl!({
            let ctx = kwargs!(
                "definition" => definition.as_str(),
                "input_args" => input_args.as_str(),
                "input_kwargs" => input_kwargs.as_str()
            );
            execute_task.funcall_public::<_, _, String>("call", (ctx,))
        });

        trace!("({id}) Result of execution: {result:?}");
        let value = result.map_err(|e| anyhow!("task execution failed: {e}")).unwrap_or("failed".to_owned());

        exe.task_awaited(task.id.clone(), value).await?;
        exe.thread_ready(thread_id).await?;
    }

    Ok(())
}
