pub use embedded_io::*;

#[cfg(feature = "nightly")]
pub mod asynch {
    pub use embedded_io_async::*;

    use crate::executor::asynch::{
        Blocker, Blocking, RawBlocking, RawTrivialUnblocking, TrivialUnblocking,
    };

    impl<B, I> ErrorType for Blocking<B, I>
    where
        I: ErrorType,
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

    impl<I> ErrorType for TrivialUnblocking<I>
    where
        I: ErrorType,
    {
        type Error = I::Error;
    }

    impl<R> Read for TrivialUnblocking<R>
    where
        R: super::Read,
    {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.api.read(buf)
        }
    }

    impl<W> Write for TrivialUnblocking<W>
    where
        W: super::Write,
    {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.api.write(buf)
        }

        async fn flush(&mut self) -> Result<(), Self::Error> {
            self.api.flush()
        }
    }

    impl<B, I> ErrorType for RawBlocking<B, I>
    where
        I: ErrorType,
    {
        type Error = I::Error;
    }

    impl<B, R> super::Read for RawBlocking<B, R>
    where
        B: Blocker,
        R: Read,
    {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            unsafe { self.blocker.as_ref() }
                .unwrap()
                .block_on(unsafe { self.api.as_mut() }.unwrap().read(buf))
        }
    }

    impl<B, W> super::Write for RawBlocking<B, W>
    where
        B: Blocker,
        W: Write,
    {
        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            unsafe { self.blocker.as_ref() }
                .unwrap()
                .block_on(unsafe { self.api.as_mut() }.unwrap().write(buf))
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            unsafe { self.blocker.as_ref() }
                .unwrap()
                .block_on(unsafe { self.api.as_mut() }.unwrap().flush())
        }
    }

    impl<I> ErrorType for RawTrivialUnblocking<I>
    where
        I: ErrorType,
    {
        type Error = I::Error;
    }

    impl<R> Read for RawTrivialUnblocking<R>
    where
        R: super::Read,
    {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            unsafe { self.api.as_mut() }.unwrap().read(buf)
        }
    }

    impl<W> Write for RawTrivialUnblocking<W>
    where
        W: super::Write,
    {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            unsafe { self.api.as_mut() }.unwrap().write(buf)
        }

        async fn flush(&mut self) -> Result<(), Self::Error> {
            unsafe { self.api.as_mut() }.unwrap().flush()
        }
    }
}
