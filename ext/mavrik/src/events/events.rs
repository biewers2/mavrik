use crate::events::{NewTask, Task, TaskId, TaskResult};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

/// A request made from a TCP client to the TCP listener service ("TCP").
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MavrikRequest {
    /// A new task being submitted.
    NewTask(NewTask),
    
    /// Wait for a task to finish and return its result.
    AwaitTask { task_id: TaskId },

    /// Request to terminate from the client.
    Terminate
}

/// A response given to a TCP client from the TCP listener service ("TCP").
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum MavrikResponse {
    /// The response for submitting a new task.
    /// Contains the created ID of the task submitted.
    NewTaskId(TaskId),
    
    /// The result of waiting for a task to complete.
    CompletedTask(TaskResult),

    /// Whether the request for termination was fulfilled.
    Terminated(bool)
}

/// Event as understood by the event loop.
///
/// All communications done between services is through events. The event loop will handle and redirect messages to
/// different components. Note that these are typically one-way communications, meaning the event loop doesn't
/// inherently support request-response type messages. A workaround for this is to provide a one-shot sender with the
/// message payload for the message receiver to send a response back to.
#[derive(Debug)]
pub enum MavrikEvent {
    /// Any general event that isn't associated w/ a specific service.
    General(GeneralEvent),
    
    /// Events that should be redirected to the task executor service ("EXE").
    Exe(ExeEvent),
    
    /// Events that should be redirected to the TCP listener service ("TCP").
    Tcp(TcpEvent),
    
    /// Events that should be redirected to the signal listener service ("SIG").
    Sig(SigEvent)
}

#[derive(Debug)]
pub enum GeneralEvent {
    Terminate
}

#[derive(Debug)]
pub enum ExeEvent {
    NewTask {
        task: Task,
        value_tx: oneshot::Sender<TaskId>
    },

    AwaitTask {
        task_id: TaskId,
        value_tx: oneshot::Sender<TaskResult>
    }
}

#[derive(Debug)]
pub enum TcpEvent {}

#[derive(Debug)]
pub enum SigEvent {}
