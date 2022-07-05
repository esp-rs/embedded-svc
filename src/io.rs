pub use embedded_io::adapters;
pub use embedded_io::blocking::*;
pub use embedded_io::*;

#[cfg(feature = "experimental")]
pub mod asynch {
    pub use embedded_io::asynch::*;
    pub use embedded_io::*;

    use crate::unblocker::asynch::{Blocker, Blocking};

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
}
