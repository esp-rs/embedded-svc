pub use embedded_io::adapters;
pub use embedded_io::blocking::*;
pub use embedded_io::*;

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    pub use embedded_io::asynch::*;
    pub use embedded_io::*;

    use crate::executor::asynch::{Blocker, Blocking, RawBlocking, RawTrivialAsync, TrivialAsync};

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
            self.blocker.block_on(self.api.read(buf))
        }
    }

    impl<B, W> super::Write for Blocking<B, W>
    where
        B: Blocker,
        W: Write,
    {
        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.blocker.block_on(self.api.write(buf))
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            self.blocker.block_on(self.api.flush())
        }
    }

    impl<I> Io for TrivialAsync<I>
    where
        I: Io,
    {
        type Error = I::Error;
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
            async move { self.api.read(buf) }
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
            async move { self.api.write(buf) }
        }

        type FlushFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<(), Self::Error>>;

        fn flush<'a>(&'a mut self) -> Self::FlushFuture<'a> {
            async move { self.api.flush() }
        }
    }

    impl<B, I> Io for RawBlocking<B, I>
    where
        I: Io,
    {
        type Error = I::Error;
    }

    impl<B, R> super::Read for RawBlocking<B, R>
    where
        B: Blocker,
        R: Read,
    {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            unsafe { self.blocker.as_ref().unwrap() }
                .block_on(unsafe { self.api.as_mut().unwrap().read(buf) })
        }
    }

    impl<B, W> super::Write for RawBlocking<B, W>
    where
        B: Blocker,
        W: Write,
    {
        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            unsafe { self.blocker.as_ref().unwrap() }
                .block_on(unsafe { self.api.as_mut().unwrap().write(buf) })
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            unsafe { self.blocker.as_ref().unwrap() }
                .block_on(unsafe { self.api.as_mut().unwrap().flush() })
        }
    }

    impl<I> Io for RawTrivialAsync<I>
    where
        I: Io,
    {
        type Error = I::Error;
    }

    impl<R> Read for RawTrivialAsync<R>
    where
        R: super::Read,
    {
        type ReadFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<usize, Self::Error>>;

        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
            async move { unsafe { self.api.as_mut().unwrap().read(buf) } }
        }
    }

    impl<W> Write for RawTrivialAsync<W>
    where
        W: super::Write,
    {
        type WriteFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<usize, Self::Error>>;

        fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
            async move { unsafe { self.api.as_mut().unwrap().write(buf) } }
        }

        type FlushFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<(), Self::Error>>;

        fn flush<'a>(&'a mut self) -> Self::FlushFuture<'a> {
            async move { unsafe { self.api.as_mut().unwrap().flush() } }
        }
    }
}
