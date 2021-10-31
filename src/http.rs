extern crate alloc;
use alloc::borrow::Cow;
use alloc::string::ToString;

pub mod client;
#[cfg(feature = "std")] // TODO: Remove
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
pub enum HttpMethod {
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

pub trait HttpHeaders {
    fn header(&self, name: impl AsRef<str>) -> Option<Cow<'_, str>>;

    fn content_type(&self) -> Option<Cow<'_, str>> {
        self.header("content-type")
    }

    fn content_len(&self) -> Option<usize> {
        self.header("content-length")
            .and_then(|v| v.as_ref().parse::<usize>().ok())
    }

    fn content_encoding(&self) -> Option<Cow<'_, str>> {
        self.header("content-encoding")
    }
}

pub trait HttpSendHeaders<'a> {
    fn set_header<H, V>(&mut self, name: H, value: V) -> &mut Self
    where
        H: Into<Cow<'a, str>>,
        V: Into<Cow<'a, str>>;

    fn set_content_type<V>(&mut self, ctype: V) -> &mut Self
    where
        V: Into<Cow<'a, str>>,
    {
        self.set_header("content-type", ctype)
    }

    fn set_content_len(&mut self, len: usize) -> &mut Self {
        self.set_header("content-length", len.to_string())
    }

    fn set_content_encoding<V>(&mut self, encoding: V) -> &mut Self
    where
        V: Into<Cow<'a, str>>,
    {
        self.set_header("content-encoding", encoding)
    }

    fn header<H, V>(mut self, name: H, value: V) -> Self
    where
        H: Into<Cow<'a, str>>,
        V: Into<Cow<'a, str>>,
        Self: Sized,
    {
        self.set_header(name, value);
        self
    }

    fn content_type<V>(mut self, ctype: V) -> Self
    where
        V: Into<Cow<'a, str>>,
        Self: Sized,
    {
        self.set_content_type(ctype);
        self
    }

    fn content_len(mut self, len: usize) -> Self
    where
        Self: Sized,
    {
        self.set_content_len(len);
        self
    }

    fn content_encoding<V>(mut self, encoding: V) -> Self
    where
        V: Into<Cow<'a, str>>,
        Self: Sized,
    {
        self.set_content_encoding(encoding);
        self
    }
}

pub trait HttpStatus {
    fn status(&self) -> u16;
    fn status_message(&self) -> Option<Cow<'_, str>>;
}

pub trait HttpSendStatus<'a> {
    fn set_ok(&mut self) -> &mut Self {
        self.set_status(200)
    }

    fn set_status(&mut self, status: u16) -> &mut Self;
    fn set_status_message<M>(&mut self, message: M) -> &mut Self
    where
        M: Into<Cow<'a, str>>;

    fn status(mut self, status: u16) -> Self
    where
        Self: Sized,
    {
        self.set_status(status);
        self
    }

    fn status_message<M>(mut self, message: M) -> Self
    where
        M: Into<Cow<'a, str>>,
        Self: Sized,
    {
        self.set_status_message(message);
        self
    }
}
