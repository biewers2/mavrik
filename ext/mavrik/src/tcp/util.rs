use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use log::trace;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

/// Read and deserialize a string from a stream.
/// 
/// The payload of the stream should contain a header of `size_of::<usize>()` bytes (called `len`). This value indicates
/// the length of the string in the stream. `len` bytes are then read from the stream into a string. This string is
/// deserialized using `serde_json`.
/// 
pub async fn read_deserialized<AR, T>(stream: &mut AR) -> Result<T, anyhow::Error>
where
    AR: AsyncRead + Unpin,
    T: DeserializeOwned + Debug
{
    let mut len_buf = [0u8; size_of::<usize>()];
    stream.read_exact(&mut len_buf).await?;
    let len = usize::from_be_bytes(len_buf);

    let mut request = vec![0u8; len];
    stream.read_exact(&mut request).await?;
    let value = serde_json::from_slice(&request)?;

    trace!(len, value:?; "Received bytes over TCP");
    Ok(value)   
}

/// Write a serialized value to a stream.
///
/// The payload of the stream contains a header of `size_of::<usize>()` bytes (called `len`). This value indicates the
/// length of the string being sent next in the stream. `len` bytes are then written to the stream as a string. This
/// string has been serialized from a generic value using `serde_json`.
/// 
pub async fn write_serialized<AW, T>(stream: &mut AW, value: T) -> Result<(), anyhow::Error>
where
    AW: AsyncWrite + Unpin,
    T: Serialize + Debug
{
    let payload = serde_json::to_string(&value)?;
    let len = payload.len();
    stream.write(&len.to_be_bytes()).await?;
    stream.write_all(payload.as_bytes()).await?;

    trace!(len, payload:?; "Sent bytes over TCP");
    Ok(())
}
