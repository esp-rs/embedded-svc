#[cfg(feature = "std")]
pub use stdio::*;

use either::*;

const BUF_SIZE: usize = 128;

pub trait Read {
    type Error;

    fn do_read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;
}

pub trait Write {
    type Error;

    fn do_write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;

    fn do_write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        let mut size = 0;

        while size < buf.len() {
            size += self.do_write(&buf[size..])?;
        }

        Ok(())
    }

    fn do_flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a, R> Read for &'a mut R
where
    R: Read,
{
    type Error = R::Error;

    fn do_read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        (*self).do_read(buf)
    }
}

impl<'a, W> Write for &'a mut W
where
    W: Write,
{
    type Error = W::Error;

    fn do_write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        (*self).do_write(buf)
    }

    fn do_flush(&mut self) -> Result<(), Self::Error> {
        (*self).do_flush()
    }
}

pub fn copy<R, W>(read: R, write: W) -> Result<u64, Either<R::Error, W::Error>>
where
    R: Read,
    W: Write,
{
    copy_len(read, write, u64::MAX)
}

pub fn copy_len<R, W>(read: R, write: W, len: u64) -> Result<u64, Either<R::Error, W::Error>>
where
    R: Read,
    W: Write,
{
    copy_len_with_progress(read, write, len, |_, _| {})
}

pub fn copy_len_with_progress<R, W>(
    mut read: R,
    mut write: W,
    mut len: u64,
    progress: impl Fn(u64, u64),
) -> Result<u64, Either<R::Error, W::Error>>
where
    R: Read,
    W: Write,
{
    let mut buf = [0_u8; BUF_SIZE];

    let mut copied = 0;

    while len > 0 {
        progress(copied, len);

        let size_read = read.do_read(&mut buf).map_err(Either::Left)?;
        if size_read == 0 {
            break;
        }

        write
            .do_write_all(&buf[0..size_read])
            .map_err(Either::Right)?;

        copied += size_read as u64;
        len -= size_read as u64;
    }

    progress(copied, len);

    Ok(copied)
}

#[cfg(feature = "std")]
mod stdio {
    pub struct StdIO<T>(pub T);

    impl<R> super::Read for StdIO<R>
    where
        R: std::io::Read,
    {
        type Error = std::io::Error;

        fn do_read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.0.read(buf)
        }
    }

    impl<W> super::Write for StdIO<W>
    where
        W: std::io::Write,
    {
        type Error = std::io::Error;

        fn do_write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.0.write(buf)
        }

        fn do_flush(&mut self) -> Result<(), Self::Error> {
            self.0.flush()
        }
    }

    impl<R> std::io::Read for StdIO<R>
    where
        R: super::Read,
        R::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
            self.0
                .do_read(buf)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        }
    }

    impl<W> std::io::Write for StdIO<W>
    where
        W: super::Write,
        W::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
            self.0
                .do_write(buf)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        }

        fn flush(&mut self) -> Result<(), std::io::Error> {
            self.0
                .do_flush()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        }
    }
}
