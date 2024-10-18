use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
use std::sync::Mutex;
use std::time::SystemTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct NewTask {
    pub queue: String,
    pub definition: String, // repr class path
    pub input_args: String, // repr JSON array
    pub input_kwargs: String, // repr JSON object
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        let (last_timestamp, last_count) = guard.deref_mut();

        // Use system timestamp as primary identifier
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        // Append counter to end in case of conflict with timestamp.
        if *last_timestamp == timestamp {
            *last_count += 1;
        } else {
            *last_timestamp = timestamp;
            *last_count = 0;
        };
        let n = *last_count;

        format!("{timestamp}-{n}")
    }
}

impl From<NewTask> for Task {
    fn from(value: NewTask) -> Self {
        let NewTask { queue, definition, input_args, input_kwargs } = value;
        let id = Self::new_id();

        Self { id, queue, definition, input_args, input_kwargs }
    }
}
