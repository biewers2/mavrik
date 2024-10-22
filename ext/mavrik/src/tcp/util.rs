use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use log::trace;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

pub async fn read_deserialized_async<AR, T>(stream: &mut AR) -> Result<T, anyhow::Error>
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

    trace!("Received {len} bytes containing {value:?} over TCP");
    Ok(value)   
}

pub async fn write_serialized_async<AW, T>(stream: &mut AW, value: T) -> Result<(), anyhow::Error>
where
    AW: AsyncWrite + Unpin,
    T: Serialize + Debug
{
    let payload = serde_json::to_string(&value)?;
    let len = payload.len();
    stream.write(&len.to_be_bytes()).await?;
    stream.write_all(payload.as_bytes()).await?;

    trace!("Sent {len} bytes containing {payload:?} over TCP");
    Ok(())
}
