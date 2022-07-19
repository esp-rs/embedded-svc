use core::fmt::{self, Debug, Display, Write as _};

use crate::io::{Read, Write};

pub use super::{Headers, Method, Query, Status};
pub use crate::io::Io;

pub trait Connection: Io {
    type Request;

    type Response;

    type Headers: Query + Headers;

    type Read: Read<Error = Self::Error>;

    type Write: Write<Error = Self::Error>;

    fn split<'a>(
        &'a mut self,
        request: &'a mut Self::Request,
    ) -> (&'a Self::Headers, &'a mut Self::Read);

    fn headers<'a>(&'a self, request: &'a Self::Request) -> &'a Self::Headers;

    fn reader<'a>(&'a mut self, request: &'a mut Self::Request) -> &'a mut Self::Read {
        let (_, read) = self.split(request);

        read
    }

    fn into_response<'a>(
        &'a mut self,
        request: Self::Request,
        status: u16,
        message: Option<&'a str>,
        headers: &'a [(&'a str, &'a str)],
    ) -> Result<Self::Response, Self::Error>;

    fn into_status_response(
        &mut self,
        request: Self::Request,
        status: u16,
    ) -> Result<Self::Response, Self::Error> {
        self.into_response(request, status, None, &[])
    }

    fn into_ok_response(&mut self, request: Self::Request) -> Result<Self::Response, Self::Error> {
        self.into_response(request, 200, Some("OK"), &[])
    }

    fn writer<'a>(&'a mut self, response: &'a mut Self::Response) -> &'a mut Self::Write;
}

pub struct HandlerError(heapless::String<128>);

impl HandlerError {
    pub fn new(message: &str) -> Self {
        Self(message.into())
    }

    pub fn message(&self) -> &str {
        &self.0
    }
}

impl<E> From<E> for HandlerError
where
    E: Debug,
{
    fn from(e: E) -> Self {
        let mut string: heapless::String<128> = "".into();

        if write!(&mut string, "{:?}", e).is_err() {
            string = "(Error string too big to serve)".into();
        }

        Self(string)
    }
}

impl Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub type HandlerResult = Result<(), HandlerError>;

pub trait Handler<C>: Send
where
    C: Connection,
{
    fn handle<'a>(&'a self, connection: &'a mut C, request: C::Request) -> HandlerResult;
}

impl<C, H> Handler<C> for &H
where
    C: Connection,
    H: Handler<C> + Send + Sync,
{
    fn handle<'a>(&'a self, connection: &'a mut C, request: C::Request) -> HandlerResult {
        (*self).handle(connection, request)
    }
}

pub struct FnHandler<F>(F);

impl<F> FnHandler<F> {
    pub const fn new<C>(f: F) -> Self
    where
        C: Connection,
        F: Fn(&mut C, C::Request) -> HandlerResult,
    {
        Self(f)
    }
}

impl<C, F> Handler<C> for FnHandler<F>
where
    C: Connection,
    F: Fn(&mut C, C::Request) -> HandlerResult + Send,
{
    fn handle<'a>(&'a self, connection: &'a mut C, request: C::Request) -> HandlerResult {
        self.0(connection, request)
    }
}

pub trait Middleware<C>: Send
where
    C: Connection,
{
    fn handle<'a, H>(
        &'a self,
        connection: &'a mut C,
        request: C::Request,
        handler: &'a H,
    ) -> HandlerResult
    where
        H: Handler<C>;

    fn compose<H>(self, handler: H) -> CompositeHandler<Self, H>
    where
        H: Handler<C>,
        Self: Sized,
    {
        CompositeHandler::new(self, handler)
    }
}

pub struct CompositeHandler<M, H> {
    middleware: M,
    handler: H,
}

impl<M, H> CompositeHandler<M, H> {
    pub fn new(middleware: M, handler: H) -> Self {
        Self {
            middleware,
            handler,
        }
    }
}

impl<M, H, C> Handler<C> for CompositeHandler<M, H>
where
    M: Middleware<C>,
    H: Handler<C>,
    C: Connection,
{
    fn handle<'a>(&'a self, connection: &'a mut C, request: C::Request) -> HandlerResult {
        self.middleware.handle(connection, request, &self.handler)
    }
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    use embedded_io::blocking::{Read as _, Write as _};

    use crate::executor::asynch::{Blocker, Blocking};
    use crate::io::{asynch::Read, asynch::Write};

    pub use super::{HandlerError, HandlerResult, Headers, Method, Query, Status};
    pub use crate::io::Io;

    pub trait Connection: Io {
        type Request;

        type Response;

        type Headers: Query + Headers;

        type Read: Read<Error = Self::Error>;

        type Write: Write<Error = Self::Error>;

        type IntoResponseFuture<'a>: Future<Output = Result<Self::Response, Self::Error>>
        where
            Self: 'a;

        fn split<'a>(
            &'a mut self,
            request: &'a mut Self::Request,
        ) -> (&'a Self::Headers, &'a mut Self::Read);

        fn headers<'a>(&'a self, request: &'a Self::Request) -> &'a Self::Headers;

        fn reader<'a>(&'a mut self, request: &'a mut Self::Request) -> &'a mut Self::Read {
            let (_, read) = self.split(request);

            read
        }

        fn into_response<'a>(
            &'a mut self,
            request: Self::Request,
            status: u16,
            message: Option<&'a str>,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::IntoResponseFuture<'a>;

        fn into_status_response<'a>(
            &'a mut self,
            request: Self::Request,
            status: u16,
        ) -> Self::IntoResponseFuture<'a> {
            self.into_response(request, status, None, &[])
        }

        fn into_ok_response<'a>(
            &'a mut self,
            request: Self::Request,
        ) -> Self::IntoResponseFuture<'a> {
            self.into_response(request, 200, None, &[])
        }

        fn writer<'a>(&'a mut self, response: &'a mut Self::Response) -> &'a mut Self::Write;
    }

    pub trait Handler<C>: Send
    where
        C: Connection,
    {
        type HandleFuture<'a>: Future<Output = HandlerResult>
        where
            Self: 'a,
            C: 'a;

        fn handle<'a>(
            &'a self,
            connection: &'a mut C,
            request: C::Request,
        ) -> Self::HandleFuture<'a>;
    }

    impl<H, C> Handler<C> for &H
    where
        C: Connection,
        H: Handler<C> + Send + Sync,
    {
        type HandleFuture<'a>
        where
            Self: 'a,
            C: 'a,
        = H::HandleFuture<'a>;

        fn handle<'a>(
            &'a self,
            connection: &'a mut C,
            request: C::Request,
        ) -> Self::HandleFuture<'a> {
            (*self).handle(connection, request)
        }
    }

    pub trait Middleware<C>: Send
    where
        C: Connection,
    {
        type HandleFuture<'a>: Future<Output = HandlerResult> + Send
        where
            Self: 'a,
            C: 'a;

        fn handle<'a, H>(
            &'a self,
            connection: &'a mut C,
            request: C::Request,
            handler: &'a H,
        ) -> Self::HandleFuture<'a>
        where
            H: Handler<C>;

        fn compose<H>(self, handler: H) -> CompositeHandler<Self, H>
        where
            H: Handler<C>,
            Self: Sized,
        {
            CompositeHandler::new(self, handler)
        }
    }

    pub struct CompositeHandler<M, H> {
        middleware: M,
        handler: H,
    }

    impl<M, H> CompositeHandler<M, H> {
        pub fn new(middleware: M, handler: H) -> Self {
            Self {
                middleware,
                handler,
            }
        }
    }

    impl<M, H, C> Handler<C> for CompositeHandler<M, H>
    where
        M: Middleware<C>,
        H: Handler<C>,
        C: Connection,
    {
        type HandleFuture<'a>
        where
            Self: 'a,
            C: 'a,
        = impl Future<Output = HandlerResult> + Send;

        fn handle<'a>(
            &'a self,
            connection: &'a mut C,
            request: C::Request,
        ) -> Self::HandleFuture<'a> {
            self.middleware.handle(connection, request, &self.handler)
        }
    }

    pub struct BlockingConnection<B, C>
    where
        C: Connection,
    {
        blocker: B,
        connection: C,
        lended_io: BlockingIo<B, C>,
    }

    impl<B, C> BlockingConnection<B, C>
    where
        C: Connection,
    {
        pub const fn new(blocker: B, connection: C) -> Self {
            Self {
                blocker,
                connection,
                lended_io: BlockingIo::None,
            }
        }
    }

    impl<B, C> Io for BlockingConnection<B, C>
    where
        C: Connection,
    {
        type Error = C::Error;
    }

    impl<B, C> super::Connection for BlockingConnection<B, C>
    where
        B: Blocker + Clone,
        C: Connection,
    {
        type Request = C::Request;

        type Response = C::Response;

        type Headers = C::Headers;

        type Read = BlockingIo<B, C>;

        type Write = BlockingIo<B, C>;

        fn split<'a>(
            &'a mut self,
            request: &'a mut Self::Request,
        ) -> (&'a Self::Headers, &'a mut Self::Read) {
            let (headers, read) = self.connection.split(request);

            self.lended_io = BlockingIo::Reader(Blocking::new(self.blocker.clone(), read));

            (headers, &mut self.lended_io)
        }

        fn headers<'a>(&'a self, request: &'a Self::Request) -> &'a Self::Headers {
            self.connection.headers(request)
        }

        fn reader<'a>(&'a mut self, request: &'a mut Self::Request) -> &'a mut Self::Read {
            let read = self.connection.reader(request);

            self.lended_io = BlockingIo::Reader(Blocking::new(self.blocker.clone(), read));

            &mut self.lended_io
        }

        fn into_response<'a>(
            &'a mut self,
            request: Self::Request,
            status: u16,
            message: Option<&'a str>,
            headers: &'a [(&'a str, &'a str)],
        ) -> Result<Self::Response, Self::Error> {
            let response = self.blocker.block_on(
                self.connection
                    .into_response(request, status, message, headers),
            )?;

            Ok(response)
        }

        fn writer<'a>(&'a mut self, response: &'a mut Self::Response) -> &'a mut Self::Write {
            let write = self.connection.writer(response);

            self.lended_io = BlockingIo::Writer(Blocking::new(self.blocker.clone(), write));

            &mut self.lended_io
        }
    }

    pub enum BlockingIo<B, C>
    where
        C: Connection,
    {
        None,
        Reader(Blocking<B, *mut C::Read>),
        Writer(Blocking<B, *mut C::Write>),
    }

    impl<B, C> Io for BlockingIo<B, C>
    where
        C: Connection,
    {
        type Error = C::Error;
    }

    impl<B, C> crate::io::Read for BlockingIo<B, C>
    where
        B: Blocker,
        C: Connection,
    {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            match self {
                Self::None => panic!(),
                Self::Reader(r) => r.0.block_on(unsafe { r.1.as_mut().unwrap() }.read(buf)),
                Self::Writer(_) => panic!(),
            }
        }
    }

    impl<B, C> crate::io::Write for BlockingIo<B, C>
    where
        B: Blocker,
        C: Connection,
    {
        fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            match self {
                Self::None => panic!(),
                Self::Reader(_) => panic!(),
                Self::Writer(w) => w.0.block_on(unsafe { w.1.as_mut().unwrap() }.write(buf)),
            }
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            match self {
                Self::None => panic!(),
                Self::Reader(_) => panic!(),
                Self::Writer(w) => w.0.block_on(unsafe { w.1.as_mut().unwrap() }.flush()),
            }
        }
    }

    // // Implement a blocking handler on top of an async handler
    // // (use case: user provides us an async handler, but we are a blocking server)
    // impl<B, H, R> super::Handler<R> for Blocking<B, H>
    // where
    //     B: Blocker + Send,
    //     H: Handler<TrivialAsync<R>>,
    //     R: super::Request,
    // {
    //     fn handle(&self, request: R) -> HandlerResult {
    //         self.0.block_on(self.1.handle(TrivialAsync::new_async(request)))
    //     }
    // }

    pub struct TrivialAsyncConnection<C>
    where
        C: super::Connection,
    {
        connection: C,
        lended_io: TrivialAsyncIo<C>,
    }

    impl<C> Io for TrivialAsyncConnection<C>
    where
        C: super::Connection,
    {
        type Error = C::Error;
    }

    impl<C> TrivialAsyncConnection<C>
    where
        C: super::Connection,
    {
        pub const fn new(connection: C) -> Self {
            Self {
                connection,
                lended_io: TrivialAsyncIo::None,
            }
        }
    }

    impl<C> Connection for TrivialAsyncConnection<C>
    where
        C: super::Connection,
    {
        type Request = C::Request;

        type Response = C::Response;

        type Headers = C::Headers;

        type Read = TrivialAsyncIo<C>;

        type Write = TrivialAsyncIo<C>;

        type IntoResponseFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<Self::Response, Self::Error>>;

        fn split<'a>(
            &'a mut self,
            request: &'a mut Self::Request,
        ) -> (&'a Self::Headers, &'a mut Self::Read) {
            let (headers, read) = self.connection.split(request);

            self.lended_io = TrivialAsyncIo::Reader(read);

            (headers, &mut self.lended_io)
        }

        fn headers<'a>(&'a self, request: &'a Self::Request) -> &'a Self::Headers {
            self.connection.headers(request)
        }

        fn into_response<'a>(
            &'a mut self,
            request: Self::Request,
            status: u16,
            message: Option<&'a str>,
            headers: &'a [(&'a str, &'a str)],
        ) -> Self::IntoResponseFuture<'a> {
            async move {
                Ok(self
                    .connection
                    .into_response(request, status, message, headers)?)
            }
        }

        fn writer<'a>(&'a mut self, response: &'a mut Self::Response) -> &'a mut Self::Write {
            let write = self.connection.writer(response);

            self.lended_io = TrivialAsyncIo::Writer(write);

            &mut self.lended_io
        }
    }

    pub enum TrivialAsyncIo<C>
    where
        C: super::Connection,
    {
        None,
        Reader(*mut C::Read),
        Writer(*mut C::Write),
    }

    impl<C> Io for TrivialAsyncIo<C>
    where
        C: super::Connection,
    {
        type Error = C::Error;
    }

    impl<C> Read for TrivialAsyncIo<C>
    where
        C: super::Connection,
    {
        type ReadFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<usize, Self::Error>>;

        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
            async move {
                match self {
                    Self::None => panic!(),
                    Self::Reader(r) => unsafe { r.as_mut().unwrap() }.read(buf),
                    Self::Writer(_) => panic!(),
                }
            }
        }
    }

    impl<C> Write for TrivialAsyncIo<C>
    where
        C: super::Connection,
    {
        type WriteFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<usize, Self::Error>>;

        type FlushFuture<'a>
        where
            Self: 'a,
        = impl Future<Output = Result<(), Self::Error>>;

        fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
            async move {
                match self {
                    Self::None => panic!(),
                    Self::Reader(_) => panic!(),
                    Self::Writer(w) => unsafe { w.as_mut().unwrap() }.write(buf),
                }
            }
        }

        fn flush(&mut self) -> Self::FlushFuture<'_> {
            async move {
                match self {
                    Self::None => panic!(),
                    Self::Reader(_) => panic!(),
                    Self::Writer(w) => unsafe { w.as_mut().unwrap() }.flush(),
                }
            }
        }
    }

    // // Implement an async handler on top of a blocking handler
    // // (use case: user provides us a blocking handler, but we are an async server,

    // impl<B, H, R> Handler<R> for Blocking<B, H>
    // where
    //     B: Blocker + Clone + Send,
    //     H: super::Handler<Blocking<B, R>>,
    //     R: Request,
    // {
    //     type HandleFuture<'a>
    //     where
    //         Self: 'a,
    //     = impl Future<Output = HandlerResult>;

    //     fn handle(&self, request: R) -> Self::HandleFuture<'_> {
    //         async move {
    //             self.1.handle(Blocking::new(self.0.clone(), request))
    //         }
    //     }
    // }
}
