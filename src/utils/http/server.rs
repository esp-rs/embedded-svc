pub mod registration {
    use crate::http::server::{Connection, Handler, HandlerResult, Method};

    pub trait HandlerRegistration<C>
    where
        C: Connection,
    {
        fn handle<'a>(
            &'a self,
            path_registered: bool,
            path: &'a str,
            method: Method,
            connection: &'a mut C,
            request: C::Request,
        ) -> HandlerResult;
    }

    impl<C> HandlerRegistration<C> for ()
    where
        C: Connection,
    {
        fn handle<'a>(
            &'a self,
            path_registered: bool,
            _path: &'a str,
            _method: Method,
            connection: &'a mut C,
            request: C::Request,
        ) -> HandlerResult {
            connection.into_status_response(request, if path_registered { 405 } else { 404 })?;

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

    impl<H, C, N> HandlerRegistration<C> for SimpleHandlerRegistration<H, N>
    where
        H: Handler<C>,
        N: HandlerRegistration<C>,
        C: Connection,
    {
        fn handle<'a>(
            &'a self,
            path_registered: bool,
            path: &'a str,
            method: Method,
            connection: &'a mut C,
            request: C::Request,
        ) -> HandlerResult {
            let path_registered2 = if self.path == path {
                if self.method == method {
                    return self.handler.handle(connection, request);
                }

                true
            } else {
                false
            };

            self.next.handle(
                path_registered || path_registered2,
                path,
                method,
                connection,
                request,
            )
        }
    }

    pub struct ServerHandler<H>(H);

    impl ServerHandler<()> {
        pub fn new() -> Self {
            Self(())
        }
    }

    impl<H> ServerHandler<H> {
        pub fn register_get<C, H2>(
            self,
            path: &'static str,
            handler: H2,
        ) -> ServerHandler<SimpleHandlerRegistration<H2, H>>
        where
            C: Connection,
            H2: Handler<C> + 'static,
        {
            self.register(path, Method::Get, handler)
        }

        pub fn register_post<C, H2>(
            self,
            path: &'static str,
            handler: H2,
        ) -> ServerHandler<SimpleHandlerRegistration<H2, H>>
        where
            C: Connection,
            H2: Handler<C> + 'static,
        {
            self.register(path, Method::Post, handler)
        }

        pub fn register_put<C, H2>(
            self,
            path: &'static str,
            handler: H2,
        ) -> ServerHandler<SimpleHandlerRegistration<H2, H>>
        where
            C: Connection,
            H2: Handler<C> + 'static,
        {
            self.register(path, Method::Put, handler)
        }

        pub fn register_delete<C, H2>(
            self,
            path: &'static str,
            handler: H2,
        ) -> ServerHandler<SimpleHandlerRegistration<H2, H>>
        where
            C: Connection,
            H2: Handler<C> + 'static,
        {
            self.register(path, Method::Delete, handler)
        }

        pub fn register<C, H2>(
            self,
            path: &'static str,
            method: Method,
            handler: H2,
        ) -> ServerHandler<SimpleHandlerRegistration<H2, H>>
        where
            C: Connection,
            H2: Handler<C> + 'static,
        {
            ServerHandler(SimpleHandlerRegistration::new(
                path, method, handler, self.0,
            ))
        }

        pub fn handle<'a, C>(
            &'a self,
            path: &'a str,
            method: Method,
            connection: &'a mut C,
            request: C::Request,
        ) -> HandlerResult
        where
            C: Connection,
            H: HandlerRegistration<C>,
        {
            self.0.handle(false, path, method, connection, request)
        }
    }

    #[cfg(feature = "experimental")]
    pub mod asynch {
        use core::future::Future;

        use crate::http::server::asynch::{Connection, Handler, HandlerResult, Method};

        pub trait HandlerRegistration<C>
        where
            C: Connection,
        {
            type HandleFuture<'a>: Future<Output = HandlerResult>
            where
                Self: 'a,
                C: 'a;

            fn handle<'a>(
                &'a self,
                path_registered: bool,
                path: &'a str,
                method: Method,
                connection: &'a mut C,
                request: C::Request,
            ) -> Self::HandleFuture<'a>;
        }

        impl<C> HandlerRegistration<C> for ()
        where
            C: Connection,
        {
            type HandleFuture<'a>
            where
                Self: 'a,
                C: 'a,
            = impl Future<Output = HandlerResult>;

            fn handle<'a>(
                &'a self,
                path_registered: bool,
                _path: &'a str,
                _method: Method,
                connection: &'a mut C,
                request: C::Request,
            ) -> Self::HandleFuture<'a> {
                async move {
                    connection
                        .into_status_response(request, if path_registered { 405 } else { 404 })
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

        impl<H, C, N> HandlerRegistration<C> for SimpleHandlerRegistration<H, N>
        where
            H: Handler<C>,
            N: HandlerRegistration<C>,
            C: Connection,
        {
            type HandleFuture<'a>
            where
                Self: 'a,
                C: 'a,
            = impl Future<Output = HandlerResult>;

            fn handle<'a>(
                &'a self,
                path_registered: bool,
                path: &'a str,
                method: Method,
                connection: &'a mut C,
                request: C::Request,
            ) -> Self::HandleFuture<'a> {
                async move {
                    let path_registered2 = if self.path == path {
                        if self.method == method {
                            return self.handler.handle(connection, request).await;
                        }

                        true
                    } else {
                        false
                    };

                    self.next
                        .handle(
                            path_registered || path_registered2,
                            path,
                            method,
                            connection,
                            request,
                        )
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
            pub fn register_get<C, H2>(
                self,
                path: &'static str,
                handler: H2,
            ) -> ServerHandler<SimpleHandlerRegistration<H2, H>>
            where
                C: Connection,
                H2: Handler<C> + 'static,
            {
                self.register(path, Method::Get, handler)
            }

            pub fn register_post<C, H2>(
                self,
                path: &'static str,
                handler: H2,
            ) -> ServerHandler<SimpleHandlerRegistration<H2, H>>
            where
                C: Connection,
                H2: Handler<C> + 'static,
            {
                self.register(path, Method::Post, handler)
            }

            pub fn register_put<C, H2>(
                self,
                path: &'static str,
                handler: H2,
            ) -> ServerHandler<SimpleHandlerRegistration<H2, H>>
            where
                C: Connection,
                H2: Handler<C> + 'static,
            {
                self.register(path, Method::Put, handler)
            }

            pub fn register_delete<C, H2>(
                self,
                path: &'static str,
                handler: H2,
            ) -> ServerHandler<SimpleHandlerRegistration<H2, H>>
            where
                C: Connection,
                H2: Handler<C> + 'static,
            {
                self.register(path, Method::Delete, handler)
            }

            pub fn register<C, H2>(
                self,
                path: &'static str,
                method: Method,
                handler: H2,
            ) -> ServerHandler<SimpleHandlerRegistration<H2, H>>
            where
                C: Connection,
                H2: Handler<C> + 'static,
            {
                ServerHandler(SimpleHandlerRegistration::new(
                    path, method, handler, self.0,
                ))
            }

            pub async fn handle<'a, C>(
                &'a self,
                path: &'a str,
                method: Method,
                connection: &'a mut C,
                request: C::Request,
            ) -> HandlerResult
            where
                C: Connection,
                H: HandlerRegistration<C>,
            {
                self.0
                    .handle(false, path, method, connection, request)
                    .await
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
