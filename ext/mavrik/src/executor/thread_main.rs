use crate::executor::{ThreadId, ThreadMessage};
use crate::messaging::{Task, TaskId, TaskResult};
use crate::rb::{mavrik_error, module_mavrik, MRHash};
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

// #[cfg(test)]
// mod tests {
//     use std::future::Future;
//     use crate::messaging::{Task, TaskId, TaskResult};
//     use crate::executor::chan::{ExecutorChannel, ThreadMessage};
//     use crate::executor::thread_main::rb_thread_main;
//     use crate::rb::in_ruby;
//     use crate::runtime::async_runtime;
//     use crate::without_gvl;
//     use anyhow::anyhow;
//     use magnus::{method, Class, Module, RHash, Ruby, Symbol};
//     use serde_json::json;
//     use tokio::sync::mpsc;
//     use crate::executor::ThreadId;
//
//     fn mock_execute_task_call(_self: magnus::Value, ctx: RHash) -> Result<String, magnus::Error> {
//         assert_eq!(ctx.fetch::<_, String>(Symbol::new("definition"))?, "TestTask".to_owned());
//         assert_eq!(ctx.fetch::<_, String>(Symbol::new("input_args"))?, "[1, 2]".to_owned());
//         assert_eq!(ctx.fetch::<_, String>(Symbol::new("input_kwargs"))?, "{\"c\": 3}".to_owned());
//
//         Ok(json!({
//             "type": "success",
//             "result": "hello, world!"
//         }).to_string())
//     }
//
//     fn ruby_harness<RbF, TF, TFut>(define_ruby: RbF, mut run_test: TF) -> Result<(), anyhow::Error>
//     where
//         RbF: FnOnce(&Ruby) -> Result<(), magnus::Error>,
//         TF: FnMut() -> TFut,
//         TFut: Future<Output = Result<(), anyhow::Error>> + Send + Sync + 'static
//     {
//         let ruby = unsafe { magnus::embed::init() };
//         define_ruby(&ruby).map_err(|e| anyhow!("{e}"))?;
//
//         without_gvl!({
//             let run_test = &mut run_test;
//             async_runtime().block_on(run_test())
//         })
//     }
//
//     fn define_ruby_constants(ruby: &Ruby) -> Result<(), magnus::Error> {
//         let module_mavrik = ruby.define_module("Mavrik")?;
//         module_mavrik.define_class("Error", ruby.exception_standard_error().as_r_class())?;
//         let class_execute_task = module_mavrik.define_class("ExecuteTask", ruby.class_object())?;
//         class_execute_task.define_method("call", method!(mock_execute_task_call, 1))?;
//         Ok(())
//     }
//
//     fn run_in_thread(thread_id: ThreadId) -> Result<(mpsc::Receiver<ThreadMessage>, mpsc::Sender<(TaskId, Task)>), anyhow::Error> {
//         in_ruby::<Result<_, anyhow::Error>>(|r| {
//             let (thread_tx, thread_rx) = mpsc::channel(1);
//             let (task_tx, task_rx) = mpsc::channel(1);
//             let exe_chan = ExecutorChannel::new(task_rx, thread_tx);
//
//             r.thread_create_from_fn(move |r| rb_thread_main(r, thread_id, exe_chan));
//
//             Ok((thread_rx, task_tx))
//         })
//     }
//
//     #[test]
//     fn sends_values_to_appropriate_channels() -> Result<(), anyhow::Error> {
//         ruby_harness(
//             define_ruby_constants,
//             || async move {
//                 let thread_id = 0usize;
//                 let task_id = TaskId::from_parts(123, 0);
//                 let task = Task {
//                     queue: "default".to_string(),
//                     ctx: json!({
//                         "definition": "TestTask",
//                         "input_args": [1, 2],
//                         "input_kwargs": {"c": 3}
//                     }).to_string()
//                 };
//
//                 let (mut thread_rx, task_tx) = run_in_thread(thread_id)?;
//
//                 assert_eq!(thread_rx.recv().await, Some(ThreadMessage::ThreadReady(thread_id)));
//                 task_tx.send((task_id, task)).await?;
//                 assert_eq!(thread_rx.recv().await, Some(ThreadMessage::Completed {
//                     task_id,
//                     task_result: TaskResult::Success {
//                         result: serde_json::Value::String("hello, world!".to_string())
//                     }
//                 }));
//                 assert_eq!(thread_rx.recv().await, Some(ThreadMessage::ThreadReady(thread_id)));
//
//                 Result::<(), anyhow::Error>::Ok(())
//             }
//         )
//     }
//
//     #[test]
//     fn handles_task_failures() -> Result<(), anyhow::Error> {
//         // ruby_harness(
//         //     define_ruby_constants,
//         //     || async move {
//         //         let thread_id = 0usize;
//         //         let task = Task {
//         //             id: "123-4".to_string(),
//         //             queue: "default".to_string(),
//         //             definition: "TestTask".to_string(),
//         //             input_args: "[1, 2]".to_string(),
//         //             input_kwargs: "{\"c\": 3}".to_string()
//         //         };
//         //
//         //         let (mut thread_rx, task_tx) = run_in_thread(thread_id)?;
//         //
//         //         assert_eq!(thread_rx.recv().await, Some(ThreadMessage::ThreadReady(thread_id)));
//         //         task_tx.send(task).await?;
//         //         assert_eq!(thread_rx.recv().await, Some(ThreadMessage::Awaited {
//         //             task_id: "123-4".to_owned(),
//         //             task_result: TaskResult::Success {
//         //                 result: "hello, world!".to_string()
//         //             }
//         //         }));
//         //         assert_eq!(thread_rx.recv().await, Some(ThreadMessage::ThreadReady(thread_id)));
//         //
//         //         Result::<(), anyhow::Error>::Ok(())
//         //     }
//         // )
//         Ok(())
//     }
// }
