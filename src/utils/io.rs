use embedded_io::Error;

use crate::io::{Read, Write};

pub fn try_read_full<R: Read>(mut read: R, buf: &mut [u8]) -> Result<usize, (R::Error, usize)> {
    let mut offset = 0;
    let mut size = 0;

    loop {
        let size_read = read.read(&mut buf[offset..]).map_err(|e| (e, size))?;

        offset += size_read;
        size += size_read;

        if size_read == 0 || size == buf.len() {
            break;
        }
    }

    Ok(size)
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CopyError<R, W> {
    Read(R),
    Write(W),
}

impl<R: core::fmt::Debug, W: core::fmt::Debug> core::fmt::Display for CopyError<R, W> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl<R: core::fmt::Debug, W: core::fmt::Debug> std::error::Error for CopyError<R, W> {}

impl<R, W> Error for CopyError<R, W>
where
    R: Error,
    W: Error,
{
    fn kind(&self) -> embedded_io::ErrorKind {
        match self {
            Self::Read(e) => e.kind(),
            Self::Write(e) => e.kind(),
        }
    }
}

pub fn copy<R, W>(read: R, write: W, buf: &mut [u8]) -> Result<u64, CopyError<R::Error, W::Error>>
where
    R: Read,
    W: Write,
{
    copy_len(read, write, buf, u64::MAX)
}

pub fn copy_len<R, W>(
    read: R,
    write: W,
    buf: &mut [u8],
    len: u64,
) -> Result<u64, CopyError<R::Error, W::Error>>
where
    R: Read,
    W: Write,
{
    copy_len_with_progress(read, write, buf, len, |_, _| {})
}

pub fn copy_len_with_progress<R, W, P>(
    mut read: R,
    mut write: W,
    buf: &mut [u8],
    mut len: u64,
    progress: P,
) -> Result<u64, CopyError<R::Error, W::Error>>
where
    R: Read,
    W: Write,
    P: Fn(u64, u64),
{
    let mut copied = 0;

    while len > 0 {
        progress(copied, len);

        let size_read = read.read(buf).map_err(CopyError::Read)?;
        if size_read == 0 {
            break;
        }

        write
            .write_all(&buf[0..size_read])
            .map_err(CopyError::Write)?;

        copied += size_read as u64;
        len -= size_read as u64;
    }

    progress(copied, len);

    Ok(copied)
}

pub mod asynch {
    use crate::io::asynch::{Read, Write};

    pub use super::CopyError;

    pub async fn try_read_full<R: Read>(
        mut read: R,
        buf: &mut [u8],
    ) -> Result<usize, (R::Error, usize)> {
        let mut offset = 0;
        let mut size = 0;

        loop {
            let size_read = read.read(&mut buf[offset..]).await.map_err(|e| (e, size))?;

            offset += size_read;
            size += size_read;

            if size_read == 0 || size == buf.len() {
                break;
            }
        }

        Ok(size)
    }

    pub async fn copy<R, W>(
        read: R,
        write: W,
        buf: &mut [u8],
    ) -> Result<u64, CopyError<R::Error, W::Error>>
    where
        R: Read,
        W: Write,
    {
        copy_len(read, write, buf, u64::MAX).await
    }

    pub async fn copy_len<R, W>(
        read: R,
        write: W,
        buf: &mut [u8],
        len: u64,
    ) -> Result<u64, CopyError<R::Error, W::Error>>
    where
        R: Read,
        W: Write,
    {
        copy_len_with_progress(read, write, buf, len, |_, _| {}).await
    }

    pub async fn copy_len_with_progress<R, W, P>(
        mut read: R,
        mut write: W,
        buf: &mut [u8],
        mut len: u64,
        progress: P,
    ) -> Result<u64, CopyError<R::Error, W::Error>>
    where
        R: Read,
        W: Write,
        P: Fn(u64, u64),
    {
        let mut copied = 0;

        while len > 0 {
            progress(copied, len);

            let size_read = read.read(buf).await.map_err(CopyError::Read)?;
            if size_read == 0 {
                break;
            }

            write
                .write_all(&buf[0..size_read])
                .await
                .map_err(CopyError::Write)?;

            copied += size_read as u64;
            len -= size_read as u64;
        }

        progress(copied, len);

        Ok(copied)
    }
}
