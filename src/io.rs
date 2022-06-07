pub use embedded_io::adapters;
pub use embedded_io::blocking::*;
pub use embedded_io::*;

use crate::errors::wrap::EitherError;

pub fn read_max<'a, R: Read>(
    mut read: R,
    buf: &'a mut [u8],
) -> Result<(&'a [u8], usize), R::Error> {
    let mut offset = 0;
    let mut size = 0;

    loop {
        let r = read.read(&mut buf[offset..])?;

        if size == 0 {
            break;
        }

        offset += r;
        size += r;
    }

    Ok((&buf[..size], size))
}

pub fn copy<const N: usize, R, W>(read: R, write: W) -> Result<u64, EitherError<R::Error, W::Error>>
where
    R: Read,
    W: Write,
{
    copy_len::<N, _, _>(read, write, u64::MAX)
}

pub fn copy_len<const N: usize, R, W>(
    read: R,
    write: W,
    len: u64,
) -> Result<u64, EitherError<R::Error, W::Error>>
where
    R: Read,
    W: Write,
{
    copy_len_with_progress::<N, _, _, _>(read, write, len, |_, _| {})
}

pub fn copy_len_with_progress<const N: usize, R, W, P>(
    mut read: R,
    mut write: W,
    mut len: u64,
    progress: P,
) -> Result<u64, EitherError<R::Error, W::Error>>
where
    R: Read,
    W: Write,
    P: Fn(u64, u64),
{
    let mut buf = [0_u8; N];

    let mut copied = 0;

    while len > 0 {
        progress(copied, len);

        let size_read = read.read(&mut buf).map_err(EitherError::E1)?;
        if size_read == 0 {
            break;
        }

        write
            .write_all(&buf[0..size_read])
            .map_err(EitherError::E2)?;

        copied += size_read as u64;
        len -= size_read as u64;
    }

    progress(copied, len);

    Ok(copied)
}

#[cfg(feature = "experimental")]
pub mod asynch {
    pub use embedded_io::*;
    //pub use embedded_io::asynch::adapters;
    pub use embedded_io::asynch::*;

    use crate::errors::wrap::EitherError;

    pub async fn read_max<'a, R: Read>(
        mut read: R,
        buf: &'a mut [u8],
    ) -> Result<(&'a [u8], usize), R::Error> {
        let mut offset = 0;
        let mut size = 0;

        loop {
            let r = read.read(&mut buf[offset..]).await?;

            if size == 0 {
                break;
            }

            offset += r;
            size += r;
        }

        Ok((&buf[..size], size))
    }

    pub async fn copy<const N: usize, R, W>(
        read: R,
        write: W,
    ) -> Result<u64, EitherError<R::Error, W::Error>>
    where
        R: Read,
        W: Write,
    {
        copy_len::<N, _, _>(read, write, u64::MAX).await
    }

    pub async fn copy_len<const N: usize, R, W>(
        read: R,
        write: W,
        len: u64,
    ) -> Result<u64, EitherError<R::Error, W::Error>>
    where
        R: Read,
        W: Write,
    {
        copy_len_with_progress::<N, _, _, _>(read, write, len, |_, _| {}).await
    }

    pub async fn copy_len_with_progress<const N: usize, R, W, P>(
        mut read: R,
        mut write: W,
        mut len: u64,
        progress: P,
    ) -> Result<u64, EitherError<R::Error, W::Error>>
    where
        R: Read,
        W: Write,
        P: Fn(u64, u64),
    {
        let mut buf = [0_u8; N];

        let mut copied = 0;

        while len > 0 {
            progress(copied, len);

            let size_read = read.read(&mut buf).await.map_err(EitherError::E1)?;
            if size_read == 0 {
                break;
            }

            write
                .write_all(&buf[0..size_read])
                .await
                .map_err(EitherError::E2)?;

            copied += size_read as u64;
            len -= size_read as u64;
        }

        progress(copied, len);

        Ok(copied)
    }
}
