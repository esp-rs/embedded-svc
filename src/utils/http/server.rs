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

        fn is_existing(&self, headers: &(impl RequestId + Headers)) -> bool;

        fn with_existing<R, F>(&self, headers: &(impl RequestId + Headers), f: F) -> Option<R>
        where
            F: FnOnce(&mut Self::SessionData) -> R;

        fn with<R, F>(
            &self,
            headers: &(impl RequestId + Headers),
            out_headers: &mut impl SendHeaders,
            f: F,
        ) -> Result<R, SessionError>
        where
            F: FnOnce(&mut Self::SessionData) -> R;

        fn invalidate(&self, req: &impl Request) -> bool;
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
        fn get_existing_id<'a>(&self, req: &'a impl Headers) -> Option<&'a str> {
            req.header("Cookie")
                .and_then(|cookies_str| Cookies::new(cookies_str).get("SESSIONID"))
        }

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

        fn is_existing(&self, req: &(impl RequestId + Headers)) -> bool {
            let current_time = (self.current_time)();
            self.cleanup(current_time);

            let id = self.get_existing_id(req);

            if let Some(id) = id {
                let mut data = self.data.lock();

                data.iter_mut()
                    .find(|entry| entry.id == id)
                    .map(|entry| entry.last_accessed = current_time)
                    .is_some()
            } else {
                false
            }
        }

        fn with_existing<R, F>(&self, headers: &(impl RequestId + Headers), f: F) -> Option<R>
        where
            F: FnOnce(&mut Self::SessionData) -> R,
        {
            let current_time = (self.current_time)();
            self.cleanup(current_time);

            let id = self.get_existing_id(headers);
            let req_id = headers.get_request_id();

            let mut data = self.data.lock();

            data.iter_mut()
                .find(|entry| Some(entry.id.as_ref()) == id || entry.id == req_id)
                .map(|entry| {
                    entry.last_accessed = current_time;
                    f(&mut entry.data)
                })
        }

        fn with<R, F>(
            &self,
            headers: &(impl RequestId + Headers),
            out_headers: &mut impl SendHeaders,
            f: F,
        ) -> Result<R, SessionError>
        where
            F: FnOnce(&mut Self::SessionData) -> R,
        {
            let current_time = (self.current_time)();
            self.cleanup(current_time);

            let id = self.get_existing_id(headers);
            let req_id = headers.get_request_id();

            let mut data = self.data.lock();

            if let Some(entry) = data
                .iter_mut()
                .find(|entry| Some(entry.id.as_ref()) == id || entry.id == req_id)
                .map(|entry| {
                    entry.last_accessed = current_time;

                    entry
                })
            {
                Ok(f(&mut entry.data))
            } else if let Some(entry) = data.iter_mut().find(|entry| entry.id == "") {
                entry.id = req_id.into();
                entry.data = Default::default();
                entry.timeout = self.default_session_timeout;
                entry.last_accessed = current_time;

                let cookies_str = headers.header("Cookie").unwrap_or("");
                let mut cookies = heapless::String::<128>::new();

                for cookie in Cookies::serialize(Cookies::set(
                    Cookies::new(cookies_str).into_iter(),
                    "SESSIONID",
                    &entry.id,
                )) {
                    cookies.push_str(cookie).unwrap(); // TODO
                }

                out_headers.set_header("Set-Cookie", &cookies);

                Ok(f(&mut entry.data))
            } else {
                Err(SessionError::MaxSessionsReachedError)
            }
        }

        fn invalidate(&self, req: &(impl RequestId + Headers)) -> bool {
            let current_time = (self.current_time)();
            self.cleanup(current_time);

            let id = self.get_existing_id(req);
            let req_id = req.get_request_id();

            let mut data = self.data.lock();

            if let Some(entry) = data
                .iter_mut()
                .find(|entry| Some(entry.id.as_ref()) == id || entry.id == req_id)
            {
                entry.id = "".into();
                true
            } else {
                false
            }
        }
    }
}
