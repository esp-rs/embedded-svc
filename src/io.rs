pub use embedded_io::adapters;
pub use embedded_io::blocking::*;
pub use embedded_io::*;

#[cfg(all(
    feature = "nightly",
    feature = "experimental",
    feature = "embedded-io-async"
))]
pub use embedded_io_3_4_compat_async::*;

#[cfg(all(feature = "nightly", feature = "experimental"))]
pub mod asynch {
    use core::future::Future;

    pub use super::embedded_io_3_async::*;
    pub use embedded_io::*;

    use crate::executor::asynch::{
        Blocker, Blocking, RawBlocking, RawTrivialUnblocking, TrivialUnblocking,
    };

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

    impl<I> Io for TrivialUnblocking<I>
    where
        I: Io,
    {
        type Error = I::Error;
    }

    impl<R> Read for TrivialUnblocking<R>
    where
        R: super::Read,
    {
        type ReadFuture<'a>
        = impl Future<Output = Result<usize, Self::Error>> + 'a where Self: 'a;

        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
            async move { self.api.read(buf) }
        }
    }

    impl<W> Write for TrivialUnblocking<W>
    where
        W: super::Write,
    {
        type WriteFuture<'a>
        = impl Future<Output = Result<usize, Self::Error>> + 'a where Self: 'a;

        fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
            async move { self.api.write(buf) }
        }

        type FlushFuture<'a>
        = impl Future<Output = Result<(), Self::Error>> + 'a where Self: 'a;

        fn flush(&mut self) -> Self::FlushFuture<'_> {
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

    impl<I> Io for RawTrivialUnblocking<I>
    where
        I: Io,
    {
        type Error = I::Error;
    }

    impl<R> Read for RawTrivialUnblocking<R>
    where
        R: super::Read,
    {
        type ReadFuture<'a>
        = impl Future<Output = Result<usize, Self::Error>> + 'a where Self: 'a;

        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
            async move { unsafe { self.api.as_mut() }.unwrap().read(buf) }
        }
    }

    impl<W> Write for RawTrivialUnblocking<W>
    where
        W: super::Write,
    {
        type WriteFuture<'a>
        = impl Future<Output = Result<usize, Self::Error>> + 'a where Self: 'a;

        fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
            async move { unsafe { self.api.as_mut() }.unwrap().write(buf) }
        }

        type FlushFuture<'a>
        = impl Future<Output = Result<(), Self::Error>> + 'a where Self: 'a;

        fn flush(&mut self) -> Self::FlushFuture<'_> {
            async move { unsafe { self.api.as_mut() }.unwrap().flush() }
        }
    }
}

#[cfg(all(feature = "nightly", feature = "experimental"))]
mod embedded_io_3_async {
    use core::future::Future;

    pub use embedded_io::blocking::ReadExactError;

    type ReadExactFuture<'a, T>
    where
        T: Read + ?Sized + 'a,
    = impl Future<Output = Result<(), ReadExactError<T::Error>>> + 'a;

    /// Async reader.
    ///
    /// Semantics are the same as [`std::io::Read`], check its documentation for details.
    pub trait Read: embedded_io::Io {
        /// Future returned by `read`.
        type ReadFuture<'a>: Future<Output = Result<usize, Self::Error>>
        where
            Self: 'a;

        /// Pull some bytes from this source into the specified buffer, returning how many bytes were read.
        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a>;

        /// Read the exact number of bytes required to fill `buf`.
        fn read_exact<'a>(&'a mut self, mut buf: &'a mut [u8]) -> ReadExactFuture<'a, Self> {
            async move {
                while !buf.is_empty() {
                    match self.read(buf).await {
                        Ok(0) => break,
                        Ok(n) => buf = &mut buf[n..],
                        Err(e) => return Err(ReadExactError::Other(e)),
                    }
                }
                if !buf.is_empty() {
                    Err(ReadExactError::UnexpectedEof)
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Async buffered reader.
    ///
    /// Semantics are the same as [`std::io::BufRead`], check its documentation for details.
    pub trait BufRead: embedded_io::Io {
        /// Future returned by `fill_buf`.
        type FillBufFuture<'a>: Future<Output = Result<&'a [u8], Self::Error>>
        where
            Self: 'a;

        /// Return the contents of the internal buffer, filling it with more data from the inner reader if it is empty.
        fn fill_buf(&mut self) -> Self::FillBufFuture<'_>;

        /// Tell this buffer that `amt` bytes have been consumed from the buffer, so they should no longer be returned in calls to `fill_buf`.
        fn consume(&mut self, amt: usize);
    }

    type WriteAllFuture<'a, T>
    where
        T: Write + ?Sized + 'a,
    = impl Future<Output = Result<(), T::Error>> + 'a;

    /// Async writer.
    ///
    /// Semantics are the same as [`std::io::Write`], check its documentation for details.
    pub trait Write: embedded_io::Io {
        /// Future returned by `write`.
        type WriteFuture<'a>: Future<Output = Result<usize, Self::Error>>
        where
            Self: 'a;

        /// Write a buffer into this writer, returning how many bytes were written.
        fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a>;

        /// Future returned by `flush`.
        type FlushFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        /// Flush this output stream, ensuring that all intermediately buffered contents reach their destination.
        fn flush(&mut self) -> Self::FlushFuture<'_>;

        /// Write an entire buffer into this writer.
        fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> WriteAllFuture<'a, Self> {
            async move {
                let mut buf = buf;
                while !buf.is_empty() {
                    match self.write(buf).await {
                        Ok(0) => panic!("zero-length write."),
                        Ok(n) => buf = &buf[n..],
                        Err(e) => return Err(e),
                    }
                }
                Ok(())
            }
        }
    }

    type RewindFuture<'a, T>
    where
        T: Seek + ?Sized + 'a,
    = impl Future<Output = Result<(), T::Error>> + 'a;

    /// Async seek within streams.
    ///
    /// Semantics are the same as [`std::io::Seek`], check its documentation for details.
    pub trait Seek: embedded_io::Io {
        /// Future returned by `seek`.
        type SeekFuture<'a>: Future<Output = Result<u64, Self::Error>>
        where
            Self: 'a;

        /// Seek to an offset, in bytes, in a stream.
        fn seek(&mut self, pos: embedded_io::SeekFrom) -> Self::SeekFuture<'_>;

        /// Rewind to the beginning of a stream.
        fn rewind(&mut self) -> RewindFuture<'_, Self> {
            async move {
                self.seek(embedded_io::SeekFrom::Start(0)).await?;
                Ok(())
            }
        }

        /// Returns the current seek position from the start of the stream.
        fn stream_position(&mut self) -> Self::SeekFuture<'_> {
            self.seek(embedded_io::SeekFrom::Current(0))
        }
    }

    impl<T: ?Sized + Read> Read for &mut T {
        type ReadFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
        where
            Self: 'a;

        #[inline]
        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
            T::read(self, buf)
        }
    }

    impl<T: ?Sized + BufRead> BufRead for &mut T {
        type FillBufFuture<'a> = impl Future<Output = Result<&'a [u8], Self::Error>>
        where
            Self: 'a;

        fn fill_buf(&mut self) -> Self::FillBufFuture<'_> {
            T::fill_buf(self)
        }

        fn consume(&mut self, amt: usize) {
            T::consume(self, amt)
        }
    }

    impl<T: ?Sized + Write> Write for &mut T {
        type WriteFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
        where
            Self: 'a;

        #[inline]
        fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
            T::write(self, buf)
        }

        type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
        where
            Self: 'a;

        #[inline]
        fn flush(&mut self) -> Self::FlushFuture<'_> {
            T::flush(self)
        }
    }

    impl<T: ?Sized + Seek> Seek for &mut T {
        type SeekFuture<'a> = impl Future<Output = Result<u64, Self::Error>> + 'a
        where
            Self: 'a;

        #[inline]
        fn seek(&mut self, pos: embedded_io::SeekFrom) -> Self::SeekFuture<'_> {
            T::seek(self, pos)
        }
    }

    /// Read is implemented for `&[u8]` by copying from the slice.
    ///
    /// Note that reading updates the slice to point to the yet unread part.
    /// The slice will be empty when EOF is reached.
    impl Read for &[u8] {
        type ReadFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
        where
            Self: 'a;

        #[inline]
        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
            async move {
                let amt = core::cmp::min(buf.len(), self.len());
                let (a, b) = self.split_at(amt);

                // First check if the amount of bytes we want to read is small:
                // `copy_from_slice` will generally expand to a call to `memcpy`, and
                // for a single byte the overhead is significant.
                if amt == 1 {
                    buf[0] = a[0];
                } else {
                    buf[..amt].copy_from_slice(a);
                }

                *self = b;
                Ok(amt)
            }
        }
    }

    impl BufRead for &[u8] {
        type FillBufFuture<'a> = impl Future<Output = Result<&'a [u8], Self::Error>>
        where
            Self: 'a;

        #[inline]
        fn fill_buf(&mut self) -> Self::FillBufFuture<'_> {
            async move { Ok(*self) }
        }

        #[inline]
        fn consume(&mut self, amt: usize) {
            *self = &self[amt..];
        }
    }

    /// Write is implemented for `&mut [u8]` by copying into the slice, overwriting
    /// its data.
    ///
    /// Note that writing updates the slice to point to the yet unwritten part.
    /// The slice will be empty when it has been completely overwritten.
    ///
    /// If the number of bytes to be written exceeds the size of the slice, write operations will
    /// return short writes: ultimately, `Ok(0)`; in this situation, `write_all` returns an error of
    /// kind `ErrorKind::WriteZero`.
    impl Write for &mut [u8] {
        type WriteFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
        where
            Self: 'a;

        #[inline]
        fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
            async move {
                let amt = core::cmp::min(buf.len(), self.len());
                let (a, b) = core::mem::take(self).split_at_mut(amt);
                a.copy_from_slice(&buf[..amt]);
                *self = b;
                Ok(amt)
            }
        }

        type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        #[inline]
        fn flush(&mut self) -> Self::FlushFuture<'_> {
            async move { Ok(()) }
        }
    }

    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
    impl<T: ?Sized + Read> Read for alloc::boxed::Box<T> {
        type ReadFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
        where
            Self: 'a;

        #[inline]
        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
            T::read(self, buf)
        }
    }

    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
    impl<T: ?Sized + BufRead> BufRead for alloc::boxed::Box<T> {
        type FillBufFuture<'a> = impl Future<Output = Result<&'a [u8], Self::Error>>
        where
            Self: 'a;

        fn fill_buf(&mut self) -> Self::FillBufFuture<'_> {
            T::fill_buf(self)
        }

        fn consume(&mut self, amt: usize) {
            T::consume(self, amt)
        }
    }

    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
    impl<T: ?Sized + Write> Write for alloc::boxed::Box<T> {
        type WriteFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
        where
            Self: 'a;

        #[inline]
        fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
            T::write(self, buf)
        }

        type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
        where
            Self: 'a;

        #[inline]
        fn flush(&mut self) -> Self::FlushFuture<'_> {
            T::flush(self)
        }
    }

    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
    impl<T: ?Sized + Seek> Seek for alloc::boxed::Box<T> {
        type SeekFuture<'a> = impl Future<Output = Result<u64, Self::Error>> + 'a
        where
            Self: 'a;

        #[inline]
        fn seek(&mut self, pos: embedded_io::SeekFrom) -> Self::SeekFuture<'_> {
            T::seek(self, pos)
        }
    }

    #[cfg(feature = "alloc")]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
    impl Write for alloc::vec::Vec<u8> {
        type WriteFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
        where
            Self: 'a;

        #[inline]
        fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
            async move {
                self.extend_from_slice(buf);
                Ok(buf.len())
            }
        }

        type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        #[inline]
        fn flush(&mut self) -> Self::FlushFuture<'_> {
            async move { Ok(()) }
        }
    }
}

#[cfg(all(
    feature = "nightly",
    feature = "experimental",
    feature = "embedded-io-async"
))]
mod embedded_io_3_4_compat_async {
    use core::future::Future;

    pub struct EmbIo<T>(pub T);

    impl<T> super::Io for EmbIo<T>
    where
        T: super::Io,
    {
        type Error = T::Error;
    }

    impl<T> super::embedded_io_3_async::Read for EmbIo<T>
    where
        T: embedded_io::asynch::Read,
    {
        type ReadFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
        where
            Self: 'a;

        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
            self.0.read(buf)
        }
    }

    impl<T> embedded_io::asynch::Read for EmbIo<T>
    where
        T: super::embedded_io_3_async::Read,
    {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.0.read(buf).await
        }
    }

    impl<T> super::embedded_io_3_async::BufRead for EmbIo<T>
    where
        T: embedded_io::asynch::BufRead,
    {
        type FillBufFuture<'a> = impl Future<Output = Result<&'a [u8], Self::Error>> + 'a
        where
            Self: 'a;

        fn fill_buf(&mut self) -> Self::FillBufFuture<'_> {
            self.0.fill_buf()
        }

        fn consume(&mut self, amt: usize) {
            self.0.consume(amt)
        }
    }

    impl<T> embedded_io::asynch::BufRead for EmbIo<T>
    where
        T: super::embedded_io_3_async::BufRead,
    {
        async fn fill_buf(&mut self) -> Result<&[u8], Self::Error> {
            self.0.fill_buf().await
        }

        fn consume(&mut self, amt: usize) {
            self.0.consume(amt)
        }
    }

    impl<T> super::embedded_io_3_async::Write for EmbIo<T>
    where
        T: embedded_io::asynch::Write,
    {
        type WriteFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
        where
            Self: 'a;

        type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
        where
            Self: 'a;

        fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
            self.0.write(buf)
        }

        fn flush(&mut self) -> Self::FlushFuture<'_> {
            self.0.flush()
        }
    }

    impl<T> embedded_io::asynch::Write for EmbIo<T>
    where
        T: super::embedded_io_3_async::Write,
    {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.0.write(buf).await
        }

        async fn flush(&mut self) -> Result<(), Self::Error> {
            self.0.flush().await
        }
    }

    impl<T> super::embedded_io_3_async::Seek for EmbIo<T>
    where
        T: embedded_io::asynch::Seek,
    {
        type SeekFuture<'a> = impl Future<Output = Result<u64, Self::Error>> + 'a
        where
            Self: 'a;

        fn seek(&mut self, pos: super::SeekFrom) -> Self::SeekFuture<'_> {
            self.0.seek(pos)
        }
    }

    impl<T> embedded_io::asynch::Seek for EmbIo<T>
    where
        T: super::embedded_io_3_async::Seek,
    {
        async fn seek(&mut self, pos: embedded_io::SeekFrom) -> Result<u64, Self::Error> {
            self.0.seek(pos).await
        }
    }
}
