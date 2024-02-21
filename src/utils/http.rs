use core::str;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum HeaderSetError {
    TooManyHeaders,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Headers<'b, const N: usize = 64>([(&'b str, &'b str); N]);

impl<'b, const N: usize> Headers<'b, N> {
    pub const fn new() -> Self {
        Self([("", ""); N])
    }

    pub fn content_len(&self) -> Option<u64> {
        self.get("Content-Length")
            .map(|content_len_str| content_len_str.parse::<u64>().unwrap())
    }

    pub fn content_type(&self) -> Option<&str> {
        self.get("Content-Type")
    }

    pub fn content_encoding(&self) -> Option<&str> {
        self.get("Content-Encoding")
    }

    pub fn transfer_encoding(&self) -> Option<&str> {
        self.get("Transfer-Encoding")
    }

    pub fn host(&self) -> Option<&str> {
        self.get("Host")
    }

    pub fn connection(&self) -> Option<&str> {
        self.get("Connection")
    }

    pub fn cache_control(&self) -> Option<&str> {
        self.get("Cache-Control")
    }

    pub fn upgrade(&self) -> Option<&str> {
        self.get("Upgrade")
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .filter(|header| !header.0.is_empty())
            .map(|header| (header.0, header.1))
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.iter()
            .find(|(hname, _)| name.eq_ignore_ascii_case(hname))
            .map(|(_, value)| value)
    }

    pub fn try_set(&mut self, name: &'b str, value: &'b str) -> Result<&mut Self, HeaderSetError> {
        for header in &mut self.0 {
            if header.0.is_empty() || header.0.eq_ignore_ascii_case(name) {
                *header = (name, value);
                return Ok(self);
            }
        }

        Err(HeaderSetError::TooManyHeaders)
    }

    pub fn set(&mut self, name: &'b str, value: &'b str) -> &mut Self {
        self.try_set(name, value).expect("No space left")
    }

    pub fn remove(&mut self, name: &str) -> &mut Self {
        let index = self
            .0
            .iter()
            .enumerate()
            .find(|(_, header)| header.0.eq_ignore_ascii_case(name));

        if let Some((mut index, _)) = index {
            while index < self.0.len() - 1 {
                self.0[index] = self.0[index + 1];

                index += 1;
            }

            self.0[index] = ("", "");
        }

        self
    }

    pub fn set_content_len(
        &mut self,
        content_len: u64,
        buf: &'b mut heapless::String<20>,
    ) -> &mut Self {
        *buf = heapless::String::<20>::try_from(content_len).unwrap();

        self.set("Content-Length", buf.as_str())
    }

    pub fn set_content_type(&mut self, content_type: &'b str) -> &mut Self {
        self.set("Content-Type", content_type)
    }

    pub fn set_content_encoding(&mut self, content_encoding: &'b str) -> &mut Self {
        self.set("Content-Encoding", content_encoding)
    }

    pub fn set_transfer_encoding(&mut self, transfer_encoding: &'b str) -> &mut Self {
        self.set("Transfer-Encoding", transfer_encoding)
    }

    pub fn set_transfer_encoding_chunked(&mut self) -> &mut Self {
        self.set_transfer_encoding("Chunked")
    }

    pub fn set_host(&mut self, host: &'b str) -> &mut Self {
        self.set("Host", host)
    }

    pub fn set_connection(&mut self, connection: &'b str) -> &mut Self {
        self.set("Connection", connection)
    }

    pub fn set_connection_close(&mut self) -> &mut Self {
        self.set_connection("Close")
    }

    pub fn set_connection_keep_alive(&mut self) -> &mut Self {
        self.set_connection("Keep-Alive")
    }

    pub fn set_connection_upgrade(&mut self) -> &mut Self {
        self.set_connection("Upgrade")
    }

    pub fn set_cache_control(&mut self, cache: &'b str) -> &mut Self {
        self.set("Cache-Control", cache)
    }

    pub fn set_cache_control_no_cache(&mut self) -> &mut Self {
        self.set_cache_control("No-Cache")
    }

    pub fn set_upgrade(&mut self, upgrade: &'b str) -> &mut Self {
        self.set("Upgrade", upgrade)
    }

    pub fn set_upgrade_websocket(&mut self) -> &mut Self {
        self.set_upgrade("websocket")
    }

    pub fn as_slice(&self) -> &[(&'b str, &'b str)] {
        let index = self
            .0
            .iter()
            .enumerate()
            .find(|(_, header)| header.0.is_empty())
            .map(|(index, _)| index)
            .unwrap_or(N);

        &self.0[..index]
    }

    pub fn release(self) -> [(&'b str, &'b str); N] {
        self.0
    }
}

impl<'b, const N: usize> Default for Headers<'b, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'b, const N: usize> crate::http::Headers for Headers<'b, N> {
    fn header(&self, name: &str) -> Option<&'_ str> {
        self.get(name)
    }
}

pub mod cookies {
    use core::iter;
    use core::str::Split;

    pub struct Cookies<'a>(&'a str);

    impl<'a> Cookies<'a> {
        pub fn new(cookies_str: &'a str) -> Self {
            Self(cookies_str)
        }

        pub fn get(&self, name: &str) -> Option<&'a str> {
            Cookies::new(self.0)
                .into_iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value)
        }

        pub fn set<'b, I>(
            iter: I,
            name: &'b str,
            value: &'b str,
        ) -> impl Iterator<Item = (&'b str, &'b str)>
        where
            I: Iterator<Item = (&'b str, &'b str)> + 'b,
        {
            iter.filter(move |(key, _)| *key != name)
                .chain(core::iter::once((name, value)))
        }

        pub fn remove<'b, I>(iter: I, name: &'b str) -> impl Iterator<Item = (&'b str, &'b str)>
        where
            I: Iterator<Item = (&'b str, &'b str)> + 'b,
        {
            iter.filter(move |(key, _)| *key != name)
        }

        pub fn serialize<'b, I>(iter: I) -> impl Iterator<Item = &'b str>
        where
            I: Iterator<Item = (&'b str, &'b str)> + 'b,
        {
            iter.flat_map(|(k, v)| {
                iter::once(";")
                    .chain(iter::once(k))
                    .chain(iter::once("="))
                    .chain(iter::once(v))
            })
            .skip(1)
        }
    }

    impl<'a> IntoIterator for Cookies<'a> {
        type Item = (&'a str, &'a str);

        type IntoIter = CookieIterator<'a>;

        fn into_iter(self) -> Self::IntoIter {
            CookieIterator::new(self.0)
        }
    }

    pub struct CookieIterator<'a>(Split<'a, char>);

    impl<'a> CookieIterator<'a> {
        pub fn new(cookies: &'a str) -> Self {
            Self(cookies.split(';'))
        }
    }

    impl<'a> Iterator for CookieIterator<'a> {
        type Item = (&'a str, &'a str);

        fn next(&mut self) -> Option<Self::Item> {
            self.0
                .next()
                .map(|cookie_pair| cookie_pair.split('='))
                .and_then(|mut cookie_pair| {
                    cookie_pair
                        .next()
                        .map(|name| cookie_pair.next().map(|value| (name, value)))
                })
                .flatten()
        }
    }
}

pub mod server {
    pub mod registration {
        use crate::http::Method;

        pub struct ChainHandler<H, N> {
            pub path: &'static str,
            pub method: Method,
            pub handler: H,
            pub next: N,
        }

        impl<H, N> ChainHandler<H, N> {
            pub fn get<H2>(
                self,
                path: &'static str,
                handler: H2,
            ) -> ChainHandler<H2, ChainHandler<H, N>> {
                self.request(path, Method::Get, handler)
            }

            pub fn post<H2>(
                self,
                path: &'static str,
                handler: H2,
            ) -> ChainHandler<H2, ChainHandler<H, N>> {
                self.request(path, Method::Post, handler)
            }

            pub fn put<H2>(
                self,
                path: &'static str,
                handler: H2,
            ) -> ChainHandler<H2, ChainHandler<H, N>> {
                self.request(path, Method::Put, handler)
            }

            pub fn delete<H2>(
                self,
                path: &'static str,
                handler: H2,
            ) -> ChainHandler<H2, ChainHandler<H, N>> {
                self.request(path, Method::Delete, handler)
            }

            pub fn request<H2>(
                self,
                path: &'static str,
                method: Method,
                handler: H2,
            ) -> ChainHandler<H2, ChainHandler<H, N>> {
                ChainHandler {
                    path,
                    method,
                    handler,
                    next: self,
                }
            }
        }

        pub struct ChainRoot;

        impl ChainRoot {
            pub fn get<H2>(self, path: &'static str, handler: H2) -> ChainHandler<H2, ChainRoot> {
                self.request(path, Method::Get, handler)
            }

            pub fn post<H2>(self, path: &'static str, handler: H2) -> ChainHandler<H2, ChainRoot> {
                self.request(path, Method::Post, handler)
            }

            pub fn put<H2>(self, path: &'static str, handler: H2) -> ChainHandler<H2, ChainRoot> {
                self.request(path, Method::Put, handler)
            }

            pub fn delete<H2>(
                self,
                path: &'static str,
                handler: H2,
            ) -> ChainHandler<H2, ChainRoot> {
                self.request(path, Method::Delete, handler)
            }

            pub fn request<H2>(
                self,
                path: &'static str,
                method: Method,
                handler: H2,
            ) -> ChainHandler<H2, ChainRoot> {
                ChainHandler {
                    path,
                    method,
                    handler,
                    next: ChainRoot,
                }
            }
        }
    }

    // TODO: Commented out as it needs a mutex, yet `embedded-svc` no longer has one
    // An option is to depend on `embassy-sync`, yet this decision would be deplayed until
    // we figure out in general what to do with the utility code in `embedded-svc`.
    // pub mod session {
    //     use core::convert::TryInto;
    //     use core::fmt;
    //     use core::time::Duration;

    //     use crate::http::server::*;

    //     use crate::utils::http::cookies::*;
    //     use crate::utils::mutex::{Mutex, RawMutex};

    //     #[derive(Debug)]
    //     #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    //     pub enum SessionError {
    //         MaxSessionsReachedError,
    //     }

    //     impl fmt::Display for SessionError {
    //         fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    //             match self {
    //                 Self::MaxSessionsReachedError => {
    //                     write!(f, "Max number of sessions reached")
    //                 }
    //             }
    //         }
    //     }

    //     #[cfg(feature = "std")]
    //     impl std::error::Error for SessionError {}

    //     pub trait Session: Send {
    //         type SessionData;

    //         fn is_existing(&self, session_id: Option<&str>) -> bool;

    //         fn with_existing<R, F>(&self, session_id: Option<&str>, f: F) -> Option<R>
    //         where
    //             F: FnOnce(&mut Self::SessionData) -> R;

    //         fn with<R, F>(&self, session_id: &str, f: F) -> Result<R, SessionError>
    //         where
    //             F: FnOnce(&mut Self::SessionData) -> R;

    //         fn invalidate(&self, session_id: Option<&str>) -> bool;
    //     }

    //     #[derive(Debug, Default)]
    //     #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    //     pub struct SessionData<S> {
    //         id: heapless::String<32>,
    //         last_accessed: Duration,
    //         timeout: Duration,
    //         data: S,
    //     }

    //     pub struct SessionImpl<M, S, T, const N: usize = 16>
    //     where
    //         M: RawMutex,
    //         S: Default + Send,
    //     {
    //         current_time: T,
    //         data: Mutex<M, [SessionData<S>; N]>,
    //         default_session_timeout: Duration,
    //     }

    //     impl<M, S, T, const N: usize> SessionImpl<M, S, T, N>
    //     where
    //         M: RawMutex,
    //         S: Default + Send,
    //     {
    //         fn cleanup(&self, current_time: Duration) {
    //             let mut data = self.data.lock();

    //             for entry in &mut *data {
    //                 if entry.last_accessed + entry.timeout < current_time {
    //                     entry.id = heapless::String::new();
    //                 }
    //             }
    //         }
    //     }

    //     impl<M, S, T, const N: usize> Session for SessionImpl<M, S, T, N>
    //     where
    //         M: RawMutex + Send + Sync,
    //         S: Default + Send,
    //         T: Fn() -> Duration + Send,
    //     {
    //         type SessionData = S;

    //         fn is_existing(&self, session_id: Option<&str>) -> bool {
    //             let current_time = (self.current_time)();
    //             self.cleanup(current_time);

    //             if let Some(session_id) = session_id {
    //                 let mut data = self.data.lock();

    //                 data.iter_mut()
    //                     .find(|entry| entry.id.as_str() == session_id)
    //                     .map(|entry| entry.last_accessed = current_time)
    //                     .is_some()
    //             } else {
    //                 false
    //             }
    //         }

    //         fn with_existing<R, F>(&self, session_id: Option<&str>, f: F) -> Option<R>
    //         where
    //             F: FnOnce(&mut Self::SessionData) -> R,
    //         {
    //             let current_time = (self.current_time)();
    //             self.cleanup(current_time);

    //             if let Some(session_id) = session_id {
    //                 let mut data = self.data.lock();

    //                 data.iter_mut()
    //                     .find(|entry| entry.id.as_str() == session_id)
    //                     .map(|entry| {
    //                         entry.last_accessed = current_time;
    //                         f(&mut entry.data)
    //                     })
    //             } else {
    //                 None
    //             }
    //         }

    //         fn with<'b, R, F>(&self, session_id: &str, f: F) -> Result<R, SessionError>
    //         where
    //             F: FnOnce(&mut Self::SessionData) -> R,
    //         {
    //             let current_time = (self.current_time)();
    //             self.cleanup(current_time);

    //             let mut data = self.data.lock();

    //             if let Some(entry) = data
    //                 .iter_mut()
    //                 .find(|entry| entry.id.as_str() == session_id)
    //                 .map(|entry| {
    //                     entry.last_accessed = current_time;

    //                     entry
    //                 })
    //             {
    //                 Ok(f(&mut entry.data))
    //             } else if let Some(entry) = data.iter_mut().find(|entry| entry.id == "") {
    //                 entry.id = session_id.try_into().unwrap();
    //                 entry.data = Default::default();
    //                 entry.timeout = self.default_session_timeout;
    //                 entry.last_accessed = current_time;

    //                 Ok(f(&mut entry.data))
    //             } else {
    //                 Err(SessionError::MaxSessionsReachedError)
    //             }
    //         }

    //         fn invalidate(&self, session_id: Option<&str>) -> bool {
    //             let current_time = (self.current_time)();
    //             self.cleanup(current_time);

    //             if let Some(session_id) = session_id {
    //                 let mut data = self.data.lock();

    //                 if let Some(entry) = data
    //                     .iter_mut()
    //                     .find(|entry| entry.id.as_str() == session_id)
    //                 {
    //                     entry.id = heapless::String::new();
    //                     true
    //                 } else {
    //                     false
    //                 }
    //             } else {
    //                 false
    //             }
    //         }
    //     }

    //     pub fn get_cookie_session_id<H>(headers: &H) -> Option<&str>
    //     where
    //         H: Headers,
    //     {
    //         headers
    //             .header("Cookie")
    //             .and_then(|cookies_str| Cookies::new(cookies_str).get("SESSIONID"))
    //     }

    //     pub fn set_cookie_session_id<'a, const N: usize, H>(
    //         headers: H,
    //         session_id: &str,
    //         cookies: &mut heapless::String<N>,
    //     ) where
    //         H: Headers + 'a,
    //     {
    //         let cookies_str = headers.header("Cookie").unwrap_or("");

    //         for cookie in Cookies::serialize(Cookies::set(
    //             Cookies::new(cookies_str).into_iter(),
    //             "SESSIONID",
    //             session_id,
    //         )) {
    //             cookies.push_str(cookie).unwrap(); // TODO
    //         }
    //     }
    // }
}
