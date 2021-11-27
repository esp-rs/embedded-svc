use super::*;

pub struct HandlerRegistration<H> {
    uri: String,
    method: Method,
    handler: H,
}

impl<H> HandlerRegistration<H> {
    pub fn new(uri: impl Into<String>, method: Method, handler: H) -> Self {
        Self {
            uri: uri.into(),
            method,
            handler,
        }
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn method(&self) -> Method {
        self.method
    }

    pub fn handler(self) -> H {
        self.handler
    }
}

pub trait Middleware<R>
where
    R: Registry,
{
    type Error: fmt::Display + fmt::Debug;

    fn handle<'a, H, E>(
        &self,
        req: R::Request<'a>,
        resp: R::Response<'a>,
        handler: &H,
    ) -> Result<Completion, Self::Error>
    where
        R: Registry,
        H: for<'b> Fn(R::Request<'b>, R::Response<'b>) -> Result<Completion, E>,
        E: fmt::Display + fmt::Debug;
}

// pub struct TestMiddleware;

// impl<R> Middleware<R> for TestMiddleware
// where
//     R: Registry,
// {
//     type Error = anyhow::Error;

//     fn handle<'a, H, E>(
//         &self,
//         req: R::Request<'a>,
//         resp: R::Response<'a>,
//         handler: &H,
//     ) -> Result<Completion, Self::Error>
//     where
//         R: Registry,
//         H: for<'b> Fn(R::Request<'b>, R::Response<'b>) -> Result<Completion, E>,
//         E: fmt::Display + fmt::Debug,
//     {
//         if req.header("foo").is_some() {
//             anyhow::bail!("Boo");
//         }

//         handler(req, resp).map_err(|e| anyhow::format_err!("{}", e))
//     }
// }

pub trait Registry: Sized {
    type Request<'a>: Request<'a>;
    type Response<'a>: Response<'a>;

    #[cfg(not(feature = "std"))]
    type Error: fmt::Debug + fmt::Display;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;

    fn set_inline_handler<H, E>(
        &mut self,
        uri: &str,
        method: Method,
        handler: H,
    ) -> Result<&mut Self, Self::Error>
    where
        H: for<'a> Fn(Self::Request<'a>, Self::Response<'a>) -> Result<Completion, E>,
        E: fmt::Display + fmt::Debug;

    fn set_handler<H, E>(
        &mut self,
        uri: &str,
        method: Method,
        handler: H,
    ) -> Result<&mut Self, Self::Error>
    where
        //Self: 'static,
        H: for<'a, 'c> Fn(&'c mut Self::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<Self as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.set_inline_handler(uri, method, into_boxed_inline_handler(handler))
    }

    fn with_middleware<M>(&mut self, middleware: M) -> MiddlewareRegistry<'_, Self, M>
    where
        M: Middleware<Self>,
    {
        MiddlewareRegistry {
            registry: self,
            middleware,
        }
    }

    fn at(&mut self, uri: impl ToString) -> HandlerRegistrationBuilder<Self> {
        HandlerRegistrationBuilder {
            uri: uri.to_string(),
            registry: self,
        }
    }

    fn register<R: FnOnce(Self) -> Result<Self, Self::Error>>(
        self,
        register: R,
    ) -> Result<Self, Self::Error> {
        register(self)
    }
}

pub struct MiddlewareRegistry<'r, RR, M> {
    registry: &'r mut RR,
    middleware: M,
}

impl<'r, R, M> Registry for MiddlewareRegistry<'r, R, M>
where
    R: Registry,
    M: Middleware<R> + Clone + 'static,
    M::Error: 'static,
{
    type Request<'a> = R::Request<'a>;
    type Response<'a> = R::Response<'a>;

    type Error = R::Error;

    fn set_inline_handler<H, E>(
        &mut self,
        uri: &str,
        method: Method,
        handler: H,
    ) -> Result<&mut Self, Self::Error>
    where
        H: for<'a> Fn(Self::Request<'a>, Self::Response<'a>) -> Result<Completion, E>,
        E: fmt::Debug + fmt::Display,
    {
        let middleware = self.middleware.clone();

        self.registry
            .set_inline_handler(uri, method, move |req, resp| {
                middleware.handle(req, resp, &handler)
            })?;

        Ok(self)
    }
}

pub struct InlineHandlerRegistrationBuilder<'r, RR> {
    uri: String,
    registry: &'r mut RR,
}

impl<'r, RR> InlineHandlerRegistrationBuilder<'r, RR>
where
    RR: Registry,
{
    pub fn get<H, E>(self, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a> Fn(RR::Request<'a>, RR::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.handler(Method::Get, handler)
    }

    pub fn put<H, E>(self, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a> Fn(RR::Request<'a>, RR::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.handler(Method::Put, handler)
    }

    pub fn post<H, E>(self, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a> Fn(RR::Request<'a>, RR::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.handler(Method::Post, handler)
    }

    pub fn delete<H, E>(self, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a> Fn(RR::Request<'a>, RR::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.handler(Method::Delete, handler)
    }

    pub fn handler<H, E>(self, method: Method, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a> Fn(RR::Request<'a>, RR::Response<'a>) -> Result<Completion, E> + 'static,
        E: fmt::Debug + fmt::Display,
    {
        self.registry
            .set_inline_handler(self.uri.as_str(), method, handler)
    }
}

pub struct HandlerRegistrationBuilder<'r, RR> {
    uri: String,
    registry: &'r mut RR,
}

impl<'r, RR> HandlerRegistrationBuilder<'r, RR>
where
    RR: Registry,
{
    pub fn inline(self) -> InlineHandlerRegistrationBuilder<'r, RR> {
        InlineHandlerRegistrationBuilder {
            uri: self.uri,
            registry: self.registry,
        }
    }

    pub fn get<H, E>(self, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a, 'c> Fn(&'c mut RR::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<RR as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.handler(Method::Get, handler)
    }

    pub fn put<H, E>(self, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a, 'c> Fn(&'c mut RR::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<RR as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.handler(Method::Put, handler)
    }

    pub fn post<H, E>(self, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a, 'c> Fn(&'c mut RR::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<RR as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.handler(Method::Post, handler)
    }

    pub fn delete<H, E>(self, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a, 'c> Fn(&'c mut RR::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<RR as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.handler(Method::Delete, handler)
    }

    pub fn handler<H, E>(self, method: Method, handler: H) -> Result<&'r mut RR, RR::Error>
    where
        H: for<'a, 'c> Fn(&'c mut RR::Request<'a>) -> Result<ResponseData, E> + 'static,
        E: fmt::Debug
            + fmt::Display
            + for<'a> From<<<RR as Registry>::Response<'a> as Response<'a>>::Error>
            + From<io::IODynError>
            + 'static,
    {
        self.registry
            .set_handler(self.uri.as_str(), method, handler)?;

        Ok(self.registry)
    }
}

fn into_boxed_inline_handler<RR, H, E>(
    handler: H,
) -> Box<dyn for<'a> Fn(RR::Request<'a>, RR::Response<'a>) -> Result<Completion, E>>
where
    RR: Registry,
    H: for<'a, 'c> Fn(&'c mut RR::Request<'a>) -> Result<ResponseData, E> + 'static,
    E: fmt::Debug
        + fmt::Display
        + for<'a> From<<<RR as Registry>::Response<'a> as Response<'a>>::Error>
        + From<io::IODynError>,
{
    Box::new(move |req, resp| handle::<RR, _, _>(req, resp, &handler))
}

fn handle<'b, RR, H, E>(
    mut req: RR::Request<'b>,
    mut inline_resp: RR::Response<'b>,
    handler: &H,
) -> Result<Completion, E>
where
    RR: Registry,
    H: for<'a, 'c> Fn(&'c mut RR::Request<'a>) -> Result<ResponseData, E>,
    E: fmt::Debug
        + fmt::Display
        + From<<<RR as Registry>::Response<'b> as Response<'b>>::Error>
        + From<io::IODynError>,
{
    let resp = handler(&mut req)?;

    inline_resp.set_status(resp.status);

    if let Some(status_message) = resp.status_message {
        inline_resp.set_status_message(status_message);
    }

    for (key, value) in resp.headers {
        inline_resp.set_header(key, value);
    }

    match resp.body {
        Body::Empty => inline_resp.submit(req).map_err(Into::into),
        Body::Bytes(bytes) => inline_resp.send_bytes(req, &bytes).map_err(Into::into),
        Body::Read(size, reader) => {
            inline_resp
                .send_reader(req, size, reader)
                .map_err(|e| match e {
                    SendError::SendError(e) => e.into(),
                    SendError::WriteError(e) => e.into(),
                })
        }
    }
}

// fn test<'a>(req: &mut impl Request<'a>) -> Result<Response, anyhow::Error> {
//     let h1 = req.header("test").unwrap();

//     let mut xxx = [0_u8; 512];
//     let mut reader = req.reader();
//     reader.do_read(&mut xxx)?;

//     let mut v: Vec<u8> = Vec::new();
//     io::StdIO(reader).read_to_end(&mut v)?;

//     Response::ok().status_message(h1.into_owned()).into()
// }
