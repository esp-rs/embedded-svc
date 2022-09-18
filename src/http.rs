pub mod client;
pub mod server;

pub mod status {
    use core::ops::Range;

    pub const INFO: Range<u16> = 100..200;
    pub const OK: Range<u16> = 200..300;
    pub const REDIRECT: Range<u16> = 300..400;
    pub const CLIENT_ERROR: Range<u16> = 400..500;
    pub const SERVER_ERROR: Range<u16> = 500..600;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Hash))]
pub enum Method {
    Delete,
    Get,
    Head,
    Post,
    Put,
    Connect,
    Options,
    Trace,
    Copy,
    Lock,
    MkCol,
    Move,
    Propfind,
    Proppatch,
    Search,
    Unlock,
    Bind,
    Rebind,
    Unbind,
    Acl,
    Report,
    MkActivity,
    Checkout,
    Merge,
    MSearch,
    Notify,
    Subscribe,
    Unsubscribe,
    Patch,
    Purge,
    MkCalendar,
    Link,
    Unlink,
}

pub trait Headers {
    fn header(&self, name: &str) -> Option<&'_ str>;

    fn content_type(&self) -> Option<&'_ str> {
        self.header("Content-Type")
    }

    fn content_len(&self) -> Option<u64> {
        self.header("Content-Length")
            .and_then(|v| v.parse::<u64>().ok())
    }

    fn content_encoding(&self) -> Option<&'_ str> {
        self.header("Content-Encoding")
    }

    fn transfer_encoding(&self) -> Option<&'_ str> {
        self.header("Transfer-Encoding")
    }

    fn connection(&self) -> Option<&'_ str> {
        self.header("Connection")
    }

    fn cache_control(&self) -> Option<&'_ str> {
        self.header("Cache-Control")
    }

    fn upgrade(&self) -> Option<&'_ str> {
        self.header("Upgrade")
    }
}

impl<H> Headers for &H
where
    H: Headers,
{
    fn header(&self, name: &str) -> Option<&'_ str> {
        (*self).header(name)
    }
}

pub trait Status {
    fn status(&self) -> u16;

    fn status_message(&self) -> Option<&'_ str>;
}

impl<S> Status for &S
where
    S: Status,
{
    fn status(&self) -> u16 {
        (*self).status()
    }

    fn status_message(&self) -> Option<&'_ str> {
        (*self).status_message()
    }
}

pub trait Query {
    fn uri(&self) -> &'_ str;

    fn method(&self) -> Method;
}

impl<Q> Query for &Q
where
    Q: Query,
{
    fn uri(&self) -> &'_ str {
        (*self).uri()
    }

    fn method(&self) -> Method {
        (*self).method()
    }
}

pub mod headers {
    pub type ContentLenParseBuf = heapless::String<20>;

    pub fn content_type(ctype: &str) -> (&str, &str) {
        ("Content-Type", ctype)
    }

    pub fn content_len(len: u64, buf: &mut ContentLenParseBuf) -> (&str, &str) {
        *buf = ContentLenParseBuf::from(len);

        ("Content-Length", buf.as_str())
    }

    pub fn content_encoding(encoding: &str) -> (&str, &str) {
        ("Content-Encoding", encoding)
    }

    pub fn transfer_encoding(encoding: &str) -> (&str, &str) {
        ("Transfer-Encoding", encoding)
    }

    pub fn transfer_encoding_chunked<'a>() -> (&'a str, &'a str) {
        transfer_encoding("Chunked")
    }

    pub fn connection(connection: &str) -> (&str, &str) {
        ("Connection", connection)
    }

    pub fn connection_upgrade<'a>() -> (&'a str, &'a str) {
        connection("Upgrade")
    }

    pub fn connection_keepalive<'a>() -> (&'a str, &'a str) {
        connection("Keep-Alive")
    }

    pub fn connection_close<'a>() -> (&'a str, &'a str) {
        connection("Close")
    }

    pub fn cache_control(cache: &str) -> (&str, &str) {
        ("Cache-Control", cache)
    }

    pub fn cache_control_no_cache<'a>() -> (&'a str, &'a str) {
        cache_control("No-Cache")
    }

    pub fn location(location: &str) -> (&str, &str) {
        ("Location", location)
    }

    pub fn upgrade(upgrade: &str) -> (&str, &str) {
        ("Upgrade", upgrade)
    }

    pub fn upgrade_websocket<'a>() -> (&'a str, &'a str) {
        upgrade("websocket")
    }
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use crate::executor::asynch::{Blocking, TrivialUnblocking};

    impl<B, Q> super::Query for Blocking<B, Q>
    where
        Q: super::Query,
    {
        fn uri(&self) -> &'_ str {
            self.api.uri()
        }

        fn method(&self) -> super::Method {
            self.api.method()
        }
    }

    impl<B, H> super::Headers for Blocking<B, H>
    where
        H: super::Headers,
    {
        fn header(&self, name: &str) -> Option<&'_ str> {
            self.api.header(name)
        }
    }

    impl<B, S> super::Status for Blocking<B, S>
    where
        S: super::Status,
    {
        fn status(&self) -> u16 {
            self.api.status()
        }

        fn status_message(&self) -> Option<&'_ str> {
            self.api.status_message()
        }
    }

    impl<Q> super::Query for TrivialUnblocking<Q>
    where
        Q: super::Query,
    {
        fn uri(&self) -> &'_ str {
            self.api.uri()
        }

        fn method(&self) -> super::Method {
            self.api.method()
        }
    }

    impl<H> super::Headers for TrivialUnblocking<H>
    where
        H: super::Headers,
    {
        fn header(&self, name: &str) -> Option<&'_ str> {
            self.api.header(name)
        }
    }

    impl<S> super::Status for TrivialUnblocking<S>
    where
        S: super::Status,
    {
        fn status(&self) -> u16 {
            self.api.status()
        }

        fn status_message(&self) -> Option<&'_ str> {
            self.api.status_message()
        }
    }
}
