use anyhow::anyhow;
use log::kv::{ToValue, Value};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct TaskId([u8; 20]);

impl TaskId {
    pub fn from_parts(timestamp: u128, count: u32) -> Self {
        let mut buf = [0u8; 20];
        (&mut buf[..16]).clone_from_slice(&timestamp.to_be_bytes());
        (&mut buf[16..]).clone_from_slice(&count.to_be_bytes());
        Self(buf)
    }

    pub fn from_split_parts(first_timestamp_part: u64, second_timestamp_part: u64, count: u32) -> Self {
        let mut buf = [0u8; 20];
        (&mut buf[..8]).clone_from_slice(&first_timestamp_part.to_be_bytes());
        (&mut buf[8..16]).clone_from_slice(&second_timestamp_part.to_be_bytes());
        (&mut buf[16..]).clone_from_slice(&count.to_be_bytes());
        Self(buf)
    }
}

impl From<&TaskId> for String {
    fn from(value: &TaskId) -> Self {
        // Serde JSON doesn't support u128 :( so we use two u64s.

        let mut time_0_buf = [0u8; 8];
        let mut time_1_buf = [0u8; 8];
        let mut count_buf = [0u8; 4];

        time_0_buf.clone_from_slice(&value.0[..8]);
        time_1_buf.clone_from_slice(&value.0[8..16]);
        count_buf.clone_from_slice(&value.0[16..]);

        let time_0: u64 = u64::from_be_bytes(time_0_buf);
        let time_1: u64 = u64::from_be_bytes(time_1_buf);
        let count: u32 = u32::from_be_bytes(count_buf);

        format!("{}-{}-{}", time_0, time_1, count)
    }
}

impl TryFrom<String> for TaskId {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut comps = value.split("-");
        let time_0_str = comps.next().ok_or(anyhow!("first part of timestamp not found"))?;
        let time_1_str = comps.next().ok_or(anyhow!("second part of timestamp not found"))?;
        let count_str = comps.next().ok_or(anyhow!("count part not found"))?;

        let time_0 = time_0_str.parse::<u64>()?;
        let time_1 = time_1_str.parse::<u64>()?;
        let count = count_str.parse::<u32>()?;

        Ok(Self::from_split_parts(time_0, time_1, count))
    }
}

impl Display for TaskId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

impl ToValue for TaskId {
    fn to_value(&self) -> Value {
        Value::from_display(self)
    }
}

impl Serialize for TaskId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let task_id_str = format!("{}", self);
        serializer.serialize_str(&task_id_str)
    }
}

impl<'de> Deserialize<'de> for TaskId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        let task_id_str = String::deserialize(deserializer)?;
        let task_id = TaskId::try_from(task_id_str).map_err(|e| Error::custom(format!("{e}")))?;
        Ok(task_id)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use crate::messaging::task_id::TaskId;

    #[test]
    fn task_id_displays() {
        let task_id = TaskId::from_parts(1729971388959, 3);
        assert_eq!(format!("{task_id}"), "0-1729971388959-3");
    }

    #[test]
    fn task_id_serializes_to_string() -> Result<(), anyhow::Error> {
        let task_id = TaskId::from_parts(1729971388959, 3);
        let task_id_str = serde_json::to_string(&task_id)?;
        assert_eq!(task_id_str, "\"0-1729971388959-3\"");
        Ok(())
    }

    #[test]
    fn task_id_deserializes_from_string() -> Result<(), anyhow::Error> {
        let task_id_str = "\"0-123-4\"";
        let task_id = serde_json::from_str::<TaskId>(&task_id_str)?;
        assert_eq!(task_id, TaskId::from_parts(123, 4));
        Ok(())
    }

    #[test]
    fn task_id_deserialization_fails_on_invalid_string() -> Result<(), anyhow::Error> {
        let task_id_str = "\"123-0\"";
        match serde_json::from_str::<TaskId>(&task_id_str) {
            Ok(_) => Err(anyhow!("expected deserialization to fail")),
            Err(e) => {
                assert_eq!(e.to_string(), "count part not found");
                Ok(())
            }
        }
    }
}
