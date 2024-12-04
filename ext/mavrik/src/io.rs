use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use log::trace;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::mem::size_of;
use anyhow::Context;

/// Read and deserialize an object from a stream.
///
/// The stream format consists of:
/// 1. Length header (usize bytes) - Size of the serialized JSON payload
/// 2. JSON payload (length bytes) - The serialized object
///
pub async fn read_object<R, T>(stream: &mut R) -> Result<T, anyhow::Error>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned + Debug
{
    let mut len_buf = [0u8; size_of::<usize>()];
    stream.read_exact(&mut len_buf).await.context("reading payload length")?;
    let len = usize::from_be_bytes(len_buf);

    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).await.context("reading payload")?;
    let object = serde_json::from_slice(&payload).context("deserializing JSON payload")?;

    trace!(len, object:?; "Read object from stream");
    Ok(object)
}

/// Write and serialize an object to a stream.
///
/// The stream format consists of:
/// 1. Length header (usize bytes) - Size of the serialized JSON payload
/// 2. JSON payload (length bytes) - The serialized object
///
pub async fn write_object<W, T>(stream: &mut W, object: T) -> Result<(), anyhow::Error>
where
    W: AsyncWrite + Unpin,
    T: Serialize + Debug
{
    let payload = serde_json::to_string(&object).context("serializing object to JSON")?;
    let len = payload.len();
    
    stream.write(&len.to_be_bytes()).await.context("writing payload length")?;
    stream.write_all(payload.as_bytes()).await.context("writing payload")?;

    trace!(len, object:?; "Wrote object to stream");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use serde::{Serialize, Deserialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestObject {
        field1: String,
        field2: i32,
    }

    #[tokio::test]
    async fn test_roundtrip() {
        let test_obj = TestObject {
            field1: "test".to_string(),
            field2: 42,
        };

        let mut buffer = Cursor::new(Vec::new());
        
        // Write object to buffer
        write_object(&mut buffer, &test_obj).await.unwrap();
        
        // Reset cursor to start
        buffer.set_position(0);
        
        // Read object back
        let read_obj: TestObject = read_object(&mut buffer).await.unwrap();
        
        assert_eq!(test_obj, read_obj);
    }
}

