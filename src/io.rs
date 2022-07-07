pub use embedded_io::adapters;
pub use embedded_io::blocking::*;
pub use embedded_io::*;

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    pub use embedded_io::asynch::*;
    pub use embedded_io::*;

    use crate::unblocker::asynch::{Blocker, Blocking, TrivialAsync};

    impl<B, I> Io for Blocking<B, I>
    where
        I: Io,
    {
        type Error = I::Error;
    }

    impl<B, R> super::Read for Blocking<B, R>
    where
        B: Blocker,
        R: Read,
    {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.0.block_on(self.1.read(buf))
        }
    }

    impl<B, W> super::Write for Blocking<B, W>
    where
        B: Blocker,
        W: Write,
    {
        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.0.block_on(self.1.write(buf))
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            self.0.block_on(self.1.flush())
        }
    }

    impl<R> Read for TrivialAsync<R>
    where
        R: super::Read,
    {
        type ReadFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<usize, Self::Error>>;

        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
            async move { self.1.read(buf) }
        }
    }

    impl<W> Write for TrivialAsync<W>
    where
        W: super::Write,
    {
        type WriteFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<usize, Self::Error>>;

        fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
            async move { self.1.write(buf) }
        }

        type FlushFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<(), Self::Error>>;

        fn flush<'a>(&'a mut self) -> Self::FlushFuture<'a> {
            async move { self.1.flush() }
        }
    }
}
