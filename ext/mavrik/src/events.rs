use std::ffi::c_int;
use std::ops::DerefMut;
use std::sync::mpsc::Sender;
use std::sync::Mutex;
use std::time::SystemTime;
use crate::rb::SubmittedTask;

#[derive(Debug)]
pub enum MavrikEvent {
    ReadyThread(ReadyThread),
    Signal(c_int),
    Task(Task)
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub queue: String,
    pub definition: String, // repr class path
    pub input_args: String, // repr JSON array
    pub input_kwargs: String, // repr JSON object
}

impl Task {
    fn new_id() -> String {
        // (timestamp, counter)
        static LAST: Mutex<(u128, usize)> = Mutex::new((0, 0));
        
        let mut guard = LAST.lock().unwrap();
        let last = guard.deref_mut();
        
        // Use system timestamp as primary identifier
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        
        // Append counter to end in case of conflict with timestamp.
        if last.0 == timestamp {
            last.1 += 1;
        } else {
            last.1 = 0;
        };
        let n = last.1;
        
        format!("{timestamp}-{n}")
    }
}

impl From<SubmittedTask> for Task {
    fn from(value: SubmittedTask) -> Self {
        let SubmittedTask { queue, definition, input_args, input_kwargs } = value;
        let id = Self::new_id();
        
        Self { id, queue, definition, input_args, input_kwargs }
    }
}

#[derive(Debug)]
pub struct ReadyThread {
    pub task_tx: Sender<Task>
}
