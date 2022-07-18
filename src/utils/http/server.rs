pub mod registration {
    use crate::http::server::{Handler, HandlerResult, Method, Request};

    pub trait HandlerRegistration<R>
    where
        R: Request,
    {
        fn handle<'a>(
            &'a self,
            path_registered: bool,
            path: &'a str,
            method: Method,
            request: R,
        ) -> HandlerResult;
    }

    impl<R> HandlerRegistration<R> for ()
    where
        R: Request,
    {
        fn handle<'a>(
            &'a self,
            path_registered: bool,
            _path: &'a str,
            _method: Method,
            request: R,
        ) -> HandlerResult {
            request.into_response(if path_registered { 405 } else { 404 }, None, &[])?;

            Ok(())
        }
    }

    pub struct SimpleHandlerRegistration<H, N> {
        path: &'static str,
        method: Method,
        handler: H,
        next: N,
    }

    impl<H, N> SimpleHandlerRegistration<H, N> {
        const fn new(path: &'static str, method: Method, handler: H, next: N) -> Self {
            Self {
                path,
                method,
                handler,
                next,
            }
        }
    }

    impl<H, R, N> HandlerRegistration<R> for SimpleHandlerRegistration<H, N>
    where
        H: Handler<R>,
        N: HandlerRegistration<R>,
        R: Request,
    {
        fn handle<'a>(
            &'a self,
            path_registered: bool,
            path: &'a str,
            method: Method,
            request: R,
        ) -> HandlerResult {
            let path_registered2 = if self.path == path {
                if self.method == method {
                    return self.handler.handle(request);
                }

                true
            } else {
                false
            };

            self.next
                .handle(path_registered || path_registered2, path, method, request)
        }
    }

    pub struct ServerHandler<H>(H);

    impl ServerHandler<()> {
        pub fn new() -> Self {
            Self(())
        }
    }

    impl<H> ServerHandler<H> {
        pub fn register<H2, R>(
            self,
            path: &'static str,
            method: Method,
            handler: H2,
        ) -> ServerHandler<SimpleHandlerRegistration<H2, H>>
        where
            H2: Handler<R> + 'static,
            R: Request,
        {
            ServerHandler(SimpleHandlerRegistration::new(
                path, method, handler, self.0,
            ))
        }

        pub async fn handle<'a, R>(
            &'a self,
            path: &'a str,
            method: Method,
            request: R,
        ) -> HandlerResult
        where
            H: HandlerRegistration<R>,
            R: Request,
        {
            self.0.handle(false, path, method, request)
        }
    }

    #[cfg(feature = "experimental")]
    pub mod asynch {
        use core::future::Future;

        use crate::http::server::asynch::{Handler, HandlerResult, Method, Request};

        pub trait HandlerRegistration<R>
        where
            R: Request,
        {
            type HandleFuture<'a>: Future<Output = HandlerResult>
            where
                Self: 'a;

            fn handle<'a>(
                &'a self,
                path_registered: bool,
                path: &'a str,
                method: Method,
                request: R,
            ) -> Self::HandleFuture<'a>;
        }

        impl<R> HandlerRegistration<R> for ()
        where
            R: Request,
        {
            type HandleFuture<'a>
            where
                Self: 'a,
            = impl Future<Output = HandlerResult>;

            fn handle<'a>(
                &'a self,
                path_registered: bool,
                _path: &'a str,
                _method: Method,
                request: R,
            ) -> Self::HandleFuture<'a> {
                async move {
                    request
                        .into_response(if path_registered { 405 } else { 404 }, None, &[])
                        .await?;

                    Ok(())
                }
            }
        }

        pub struct SimpleHandlerRegistration<H, N> {
            path: &'static str,
            method: Method,
            handler: H,
            next: N,
        }

        impl<H, N> SimpleHandlerRegistration<H, N> {
            const fn new(path: &'static str, method: Method, handler: H, next: N) -> Self {
                Self {
                    path,
                    method,
                    handler,
                    next,
                }
            }
        }

        impl<H, R, N> HandlerRegistration<R> for SimpleHandlerRegistration<H, N>
        where
            H: Handler<R>,
            N: HandlerRegistration<R>,
            R: Request,
        {
            type HandleFuture<'a>
            where
                Self: 'a,
            = impl Future<Output = HandlerResult>;

            fn handle<'a>(
                &'a self,
                path_registered: bool,
                path: &'a str,
                method: Method,
                request: R,
            ) -> Self::HandleFuture<'a> {
                async move {
                    let path_registered2 = if self.path == path {
                        if self.method == method {
                            return self.handler.handle(request).await;
                        }

                        true
                    } else {
                        false
                    };

                    self.next
                        .handle(path_registered || path_registered2, path, method, request)
                        .await
                }
            }
        }

        pub struct ServerHandler<H>(H);

        impl ServerHandler<()> {
            pub fn new() -> Self {
                Self(())
            }
        }

        impl<H> ServerHandler<H> {
            pub fn register<H2, R>(
                self,
                path: &'static str,
                method: Method,
                handler: H2,
            ) -> ServerHandler<SimpleHandlerRegistration<H2, H>>
            where
                H2: Handler<R> + 'static,
                R: Request,
            {
                ServerHandler(SimpleHandlerRegistration::new(
                    path, method, handler, self.0,
                ))
            }

            pub async fn handle<'a, R>(
                &'a self,
                path: &'a str,
                method: Method,
                request: R,
            ) -> HandlerResult
            where
                H: HandlerRegistration<R>,
                R: Request,
            {
                self.0.handle(false, path, method, request).await
            }
        }
    }
}

pub mod session {
    use core::fmt;
    use core::time::Duration;

    use crate::http::cookies::*;
    use crate::http::server::*;
    use crate::mutex::*;

    #[derive(Debug)]
    pub enum SessionError {
        MaxSessionsReachedError,
    }

    impl fmt::Display for SessionError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                SessionError::MaxSessionsReachedError => {
                    write!(f, "Max number of sessions reached")
                }
            }
        }
    }

    #[cfg(feature = "std")]
    impl std::error::Error for SessionError {}

    pub trait Session: Send {
        type SessionData;

        fn is_existing(&self, session_id: Option<&str>) -> bool;

        fn with_existing<R, F>(&self, session_id: Option<&str>, f: F) -> Option<R>
        where
            F: FnOnce(&mut Self::SessionData) -> R;

        fn with<R, F>(&self, session_id: &str, f: F) -> Result<R, SessionError>
        where
            F: FnOnce(&mut Self::SessionData) -> R;

        fn invalidate(&self, session_id: Option<&str>) -> bool;
    }

    #[derive(Debug, Default)]
    pub struct SessionData<S> {
        id: heapless::String<32>,
        last_accessed: Duration,
        timeout: Duration,
        data: S,
    }

    pub struct SessionImpl<M, S, T, const N: usize = 16>
    where
        M: Mutex<Data = [SessionData<S>; N]>,
        S: Default,
    {
        current_time: T,
        data: M,
        default_session_timeout: Duration,
    }

    impl<M, S, T, const N: usize> SessionImpl<M, S, T, N>
    where
        M: Mutex<Data = [SessionData<S>; N]>,
        S: Default,
    {
        fn cleanup(&self, current_time: Duration) {
            let mut data = self.data.lock();

            for entry in &mut *data {
                if entry.last_accessed + entry.timeout < current_time {
                    entry.id = "".into();
                }
            }
        }
    }

    impl<M, S, T, const N: usize> Session for SessionImpl<M, S, T, N>
    where
        M: Mutex<Data = [SessionData<S>; N]> + Send,
        S: Default,
        T: Fn() -> Duration + Send,
    {
        type SessionData = S;

        fn is_existing(&self, session_id: Option<&str>) -> bool {
            let current_time = (self.current_time)();
            self.cleanup(current_time);

            if let Some(session_id) = session_id {
                let mut data = self.data.lock();

                data.iter_mut()
                    .find(|entry| entry.id.as_str() == session_id)
                    .map(|entry| entry.last_accessed = current_time)
                    .is_some()
            } else {
                false
            }
        }

        fn with_existing<R, F>(&self, session_id: Option<&str>, f: F) -> Option<R>
        where
            F: FnOnce(&mut Self::SessionData) -> R,
        {
            let current_time = (self.current_time)();
            self.cleanup(current_time);

            if let Some(session_id) = session_id {
                let mut data = self.data.lock();

                data.iter_mut()
                    .find(|entry| entry.id.as_str() == session_id)
                    .map(|entry| {
                        entry.last_accessed = current_time;
                        f(&mut entry.data)
                    })
            } else {
                None
            }
        }

        fn with<'b, R, F>(&self, session_id: &str, f: F) -> Result<R, SessionError>
        where
            F: FnOnce(&mut Self::SessionData) -> R,
        {
            let current_time = (self.current_time)();
            self.cleanup(current_time);

            let mut data = self.data.lock();

            if let Some(entry) = data
                .iter_mut()
                .find(|entry| entry.id.as_str() == session_id)
                .map(|entry| {
                    entry.last_accessed = current_time;

                    entry
                })
            {
                Ok(f(&mut entry.data))
            } else if let Some(entry) = data.iter_mut().find(|entry| entry.id == "") {
                entry.id = session_id.into();
                entry.data = Default::default();
                entry.timeout = self.default_session_timeout;
                entry.last_accessed = current_time;

                Ok(f(&mut entry.data))
            } else {
                Err(SessionError::MaxSessionsReachedError)
            }
        }

        fn invalidate(&self, session_id: Option<&str>) -> bool {
            let current_time = (self.current_time)();
            self.cleanup(current_time);

            if let Some(session_id) = session_id {
                let mut data = self.data.lock();

                if let Some(entry) = data
                    .iter_mut()
                    .find(|entry| entry.id.as_str() == session_id)
                {
                    entry.id = "".into();
                    true
                } else {
                    false
                }
            } else {
                false
            }
        }
    }

    pub fn get_cookie_session_id<'a, H>(headers: &'a H) -> Option<&'a str>
    where
        H: Headers,
    {
        headers
            .header("Cookie")
            .and_then(|cookies_str| Cookies::new(cookies_str).get("SESSIONID"))
    }

    pub fn set_cookie_session_id<'a, const N: usize, H>(
        headers: H,
        session_id: &str,
        cookies: &mut heapless::String<N>,
    ) where
        H: Headers + 'a,
    {
        let cookies_str = headers.header("Cookie").unwrap_or("");

        for cookie in Cookies::serialize(Cookies::set(
            Cookies::new(cookies_str).into_iter(),
            "SESSIONID",
            session_id,
        )) {
            cookies.push_str(cookie).unwrap(); // TODO
        }
    }
}
