use core::fmt;
use core::result::Result;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

#[cfg(feature = "std")]
pub use stdio::*;

use crate::service::Service;

const BUF_SIZE: usize = 64;

#[cfg(feature = "std")]
pub type IODynError = std::io::Error;

#[cfg(not(feature = "std"))]
#[derive(Debug)]
pub struct IODynError(i32);

#[cfg(not(feature = "std"))]
impl fmt::Display for IODynError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IO Error {}", self.0)
    }
}

pub trait Read: Service {
    fn do_read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    #[cfg(feature = "alloc")]
    fn into_dyn_read(self) -> Box<dyn Read<Error = IODynError>>
    where
        Self: Sized + 'static,
        Self::Error: Into<IODynError>,
    {
        Box::new(DynIO(self))
    }
}

pub trait Write: Service {
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

    #[cfg(feature = "alloc")]
    fn into_dyn_write(self) -> Box<dyn Write<Error = IODynError>>
    where
        Self: Sized + 'static,
        Self::Error: Into<IODynError>,
    {
        Box::new(DynIO(self))
    }
}

impl<'a, R> Read for &'a mut R
where
    R: Read,
{
    fn do_read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        (*self).do_read(buf)
    }
}

#[cfg(feature = "alloc")]
impl Service for Box<dyn Read<Error = IODynError>> {
    type Error = IODynError;
}

#[cfg(feature = "alloc")]
impl Read for Box<dyn Read<Error = IODynError>> {
    fn do_read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.as_mut().do_read(buf)
    }
}

impl<'a, W> Write for &'a mut W
where
    W: Write,
{
    fn do_write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        (*self).do_write(buf)
    }

    fn do_flush(&mut self) -> Result<(), Self::Error> {
        (*self).do_flush()
    }
}

#[cfg(feature = "alloc")]
impl Service for Box<dyn Write<Error = IODynError>> {
    type Error = IODynError;
}

#[cfg(feature = "alloc")]
impl Write for Box<dyn Write<Error = IODynError>> {
    fn do_write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.as_mut().do_write(buf)
    }

    fn do_flush(&mut self) -> Result<(), Self::Error> {
        self.as_mut().do_flush()
    }
}

struct DynIO<S>(S);

impl<S> Service for DynIO<S>
where
    S: Service,
    S::Error: Into<IODynError>,
{
    type Error = IODynError;
}

impl<R> Read for DynIO<R>
where
    R: Read,
    R::Error: Into<IODynError>,
{
    fn do_read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.0.do_read(buf).map_err(Into::into)
    }
}

impl<W> Write for DynIO<W>
where
    W: Write,
    W::Error: Into<IODynError>,
{
    fn do_write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.0.do_write(buf).map_err(Into::into)
    }

    fn do_flush(&mut self) -> Result<(), Self::Error> {
        self.0.do_flush().map_err(Into::into)
    }
}

pub struct Bytes<R, const N: usize> {
    reader: R,
    buf: [u8; N],
    index: usize,
    read: usize,
}

impl<R, const N: usize> Bytes<R, N>
where
    R: Read,
{
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buf: [0_u8; N],
            index: 1,
            read: 1,
        }
    }
}

impl<R, const N: usize> Iterator for Bytes<R, N>
where
    R: Read,
{
    type Item = Result<u8, R::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.read && self.read > 0 {
            match self.reader.do_read(&mut self.buf) {
                Err(e) => return Some(Err(e)),
                Ok(read) => {
                    self.read = read;
                    self.index = 0;
                }
            }
        }

        if self.read == 0 {
            None
        } else {
            let result = self.buf[self.index];
            self.index += 1;

            Some(Ok(result))
        }
    }
}

#[derive(Debug)]
pub enum CopyError<R, W>
where
    R: fmt::Display + fmt::Debug,
    W: fmt::Display + fmt::Debug,
{
    ReadError(R),
    WriteError(W),
}

impl<R, W> fmt::Display for CopyError<R, W>
where
    R: fmt::Display + fmt::Debug,
    W: fmt::Display + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CopyError::ReadError(r) => write!(f, "Read Error {}", r),
            CopyError::WriteError(w) => write!(f, "Write Error {}", w),
        }
    }
}

#[cfg(feature = "std")]
impl<R, W> std::error::Error for CopyError<R, W>
where
    R: fmt::Display + fmt::Debug,
    W: fmt::Display + fmt::Debug,
    // TODO
    // where
    //     R: std::error::Error + 'static,
    //     W: std::error::Error + 'static,
{
    // TODO
    // fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    //     match self {
    //         CopyError::ReadError(r) => Some(r),
    //         CopyError::WriteError(w) => Some(w),
    //     }
    // }
}

pub fn copy<R, W>(read: R, write: W) -> Result<u64, CopyError<R::Error, W::Error>>
where
    R: Read,
    W: Write,
{
    copy_len(read, write, u64::MAX)
}

pub fn copy_len<R, W>(read: R, write: W, len: u64) -> Result<u64, CopyError<R::Error, W::Error>>
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
) -> Result<u64, CopyError<R::Error, W::Error>>
where
    R: Read,
    W: Write,
{
    let mut buf = [0_u8; BUF_SIZE];

    let mut copied = 0;

    while len > 0 {
        progress(copied, len);

        let size_read = read.do_read(&mut buf).map_err(CopyError::ReadError)?;
        if size_read == 0 {
            break;
        }

        write
            .do_write_all(&buf[0..size_read])
            .map_err(CopyError::WriteError)?;

        copied += size_read as u64;
        len -= size_read as u64;
    }

    progress(copied, len);

    Ok(copied)
}

#[cfg(feature = "std")]
mod stdio {
    pub struct StdRead<T>(pub T);

    impl<R> crate::service::Service for StdRead<R>
    where
        R: std::io::Read,
    {
        type Error = std::io::Error;
    }

    impl<R> super::Read for StdRead<R>
    where
        R: std::io::Read,
    {
        fn do_read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.0.read(buf)
        }
    }

    impl<R> std::io::Read for StdRead<R>
    where
        R: super::Read,
    {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
            self.0
                .do_read(buf)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        }
    }

    pub struct StdWrite<T>(pub T);

    impl<W> crate::service::Service for StdWrite<W>
    where
        W: std::io::Write,
    {
        type Error = std::io::Error;
    }

    impl<W> super::Write for StdWrite<W>
    where
        W: std::io::Write,
    {
        fn do_write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.0.write(buf)
        }

        fn do_flush(&mut self) -> Result<(), Self::Error> {
            self.0.flush()
        }
    }

    impl<W> std::io::Write for StdWrite<W>
    where
        W: super::Write,
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
