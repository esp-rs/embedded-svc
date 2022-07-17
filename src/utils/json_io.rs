use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::io::{Read, Write};
use crate::utils::io::*;

#[derive(Debug)]
pub enum SerdeError<E> {
    IoError(E),
    SerdeError,
}

#[cfg(feature = "json_io")]
pub fn read_buf<'a, R, T>(read: R, buf: &'a mut [u8]) -> Result<T, SerdeError<R::Error>>
where
    R: Read,
    T: Deserialize<'a>,
{
    let read_len = try_read_full(read, buf).map_err(|(e, _)| SerdeError::IoError(e))?;

    let result = serde_json::from_slice(&buf[..read_len]).map_err(|_| SerdeError::SerdeError)?;

    Ok(result)
}

#[cfg(feature = "json_io")]
pub fn read<const N: usize, R, T>(read: R) -> Result<T, SerdeError<R::Error>>
where
    R: Read,
    T: DeserializeOwned,
{
    let mut buf = [0_u8; N];

    let read_len = try_read_full(read, &mut buf).map_err(|(e, _)| SerdeError::IoError(e))?;

    let result = serde_json::from_slice(&buf[..read_len]).map_err(|_| SerdeError::SerdeError)?;

    Ok(result)
}

#[cfg(feature = "json_io")]
pub fn write<const N: usize, W, T>(mut write: W, value: &T) -> Result<(), SerdeError<W::Error>>
where
    W: Write,
    T: Serialize,
{
    let vec = serde_json::to_vec(value).map_err(|_| SerdeError::SerdeError)?;

    write.write_all(&vec).map_err(SerdeError::IoError)
}

#[cfg(feature = "json_io")]
pub fn response<const N: usize, R, T>(request: R, value: &T) -> Result<(), SerdeError<R::Error>>
where
    R: crate::http::server::Request,
    T: Serialize,
{
    let mut writer = request
        .into_response(
            200,
            None,
            core::iter::once(("Content-Type", "application/json")),
        )
        .map_err(SerdeError::IoError)?;

    write::<N, _, _>(&mut writer, value)?;

    Ok(())
}

pub mod asynch {
    use serde::{de::DeserializeOwned, Deserialize, Serialize};

    use crate::io::asynch::{Read, Write};
    use crate::utils::io::asynch::*;

    pub use super::SerdeError;

    #[cfg(feature = "json_io")]
    pub async fn read_buf<'a, R, T>(read: R, buf: &'a mut [u8]) -> Result<T, SerdeError<R::Error>>
    where
        R: Read,
        T: Deserialize<'a>,
    {
        let read_len = try_read_full(read, buf)
            .await
            .map_err(|(e, _)| SerdeError::IoError(e))?;

        let result =
            serde_json::from_slice(&buf[..read_len]).map_err(|_| SerdeError::SerdeError)?;

        Ok(result)
    }

    #[cfg(feature = "json_io")]
    pub async fn read<const N: usize, R, T>(read: R) -> Result<T, SerdeError<R::Error>>
    where
        R: Read,
        T: DeserializeOwned,
    {
        let mut buf = [0_u8; N];

        let read_len = try_read_full(read, &mut buf)
            .await
            .map_err(|(e, _)| SerdeError::IoError(e))?;

        let result =
            serde_json::from_slice(&buf[..read_len]).map_err(|_| SerdeError::SerdeError)?;

        Ok(result)
    }

    #[cfg(feature = "json_io")]
    pub async fn write<const N: usize, W, T>(
        mut write: W,
        value: &T,
    ) -> Result<(), SerdeError<W::Error>>
    where
        W: Write,
        T: Serialize,
    {
        let vec = serde_json::to_vec(value).map_err(|_| SerdeError::SerdeError)?;

        write.write_all(&vec).await.map_err(SerdeError::IoError)
    }
}
