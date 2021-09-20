const BUF_SIZE: usize = 128;

pub trait Read {
    type Error;

    fn do_read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    fn do_copy<W>(&mut self, write: &mut W) -> Result<u64, Self::Error>
    where
        W: Write<Error = Self::Error>,
    {
        self.do_copy_len(u64::MAX, write)
    }

    fn do_copy_len<W>(&mut self, mut len: u64, write: &mut W) -> Result<u64, Self::Error>
    where
        W: Write<Error = Self::Error>,
    {
        let mut buf = [0_u8; BUF_SIZE];

        let mut copied = 0;

        while len > 0 {
            let size_read = self.do_read(&mut buf)?;
            if size_read == 0 {
                break;
            }

            write.do_write_all(&buf[0..size_read])?;

            copied += size_read as u64;
            len -= size_read as u64;
        }

        Ok(copied)
    }
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

#[cfg(feature = "std")]
impl<R> Read for R
where
    R: std::io::Read,
{
    type Error = std::io::Error;

    fn do_read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.read(buf)
    }
}

#[cfg(feature = "std")]
impl<W> Write for W
where
    W: std::io::Write,
{
    type Error = std::io::Error;

    fn do_write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.write(buf)
    }

    fn do_flush(&mut self) -> Result<(), Self::Error> {
        self.flush()
    }
}

#[cfg(feature = "std")]
pub struct StdIO<'a, T>(pub &'a mut T);

#[cfg(feature = "std")]
impl<'a, R> std::io::Read for StdIO<'a, R>
where
    R: Read,
    R::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        self.0
            .do_read(buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}

#[cfg(feature = "std")]
impl<'a, W> std::io::Write for StdIO<'a, W>
where
    W: Write,
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
