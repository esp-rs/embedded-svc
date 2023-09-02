pub use embedded_io::*;

#[cfg(feature = "nightly")]
pub mod asynch {
    pub use embedded_io_async::*;
}
