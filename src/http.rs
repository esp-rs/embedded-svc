extern crate alloc;
use alloc::borrow::Cow;

pub mod client;
pub mod server;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

pub trait SendHeaders<'a>: Sized {
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
    {
        self.set_header(name, value);
        self
    }

    fn content_type<V>(mut self, ctype: V) -> Self
    where
        V: Into<Cow<'a, str>>,
    {
        self.set_content_type(ctype);
        self
    }

    fn content_len(mut self, len: usize) -> Self {
        self.set_content_len(len);
        self
    }

    fn content_encoding<V>(mut self, encoding: V) -> Self
    where
        V: Into<Cow<'a, str>>,
    {
        self.set_content_encoding(encoding);
        self
    }
}

pub trait Status {
    fn status(&self) -> u16;
    fn status_message(&self) -> Option<Cow<'_, str>>;
}

pub trait SendStatus<'a>: Sized {
    fn set_ok(&mut self) -> &mut Self {
        self.set_status(200)
    }

    fn set_status(&mut self, status: u16) -> &mut Self;
    fn set_status_message<M>(&mut self, message: M) -> &mut Self
    where
        M: Into<Cow<'a, str>>;

    fn status(mut self, status: u16) -> Self {
        self.set_status(status);
        self
    }

    fn status_message<M>(mut self, message: M) -> Self
    where
        M: Into<Cow<'a, str>>,
    {
        self.set_status_message(message);
        self
    }
}
