use core::fmt::Write;

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

    fn content_len(&self) -> Option<usize> {
        self.header("Content-Length")
            .and_then(|v| v.parse::<usize>().ok())
    }

    fn content_encoding(&self) -> Option<&'_ str> {
        self.header("Content-Encoding")
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

pub trait SendHeaders {
    fn set_header(&mut self, name: &str, value: &str) -> &mut Self;

    fn set_content_type(&mut self, ctype: &str) -> &mut Self {
        self.set_header("Content-Type", ctype)
    }

    fn set_content_len(&mut self, len: usize) -> &mut Self {
        let mut buf: heapless::String<32> = "".into();

        write!(&mut buf, "{}", len).unwrap();

        self.set_header("Content-Length", &buf)
    }

    fn set_content_encoding(&mut self, encoding: &str) -> &mut Self {
        self.set_header("Content-Encoding", encoding)
    }

    fn set_redirect(&mut self, location: &str) -> &mut Self {
        self.set_header("Location", location)
    }

    fn header<H, V>(mut self, name: H, value: V) -> Self
    where
        H: AsRef<str>,
        V: AsRef<str>,
        Self: Sized,
    {
        self.set_header(name.as_ref(), value.as_ref());
        self
    }

    fn content_type<V>(mut self, ctype: V) -> Self
    where
        V: AsRef<str>,
        Self: Sized,
    {
        self.set_content_type(ctype.as_ref());
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
        V: AsRef<str>,
        Self: Sized,
    {
        self.set_content_encoding(encoding.as_ref());
        self
    }

    fn redirect<V>(mut self, location: V) -> Self
    where
        V: AsRef<str>,
        Self: Sized,
    {
        self.set_header("location", location.as_ref());
        self
    }
}

impl<S> SendHeaders for &mut S
where
    S: SendHeaders,
{
    fn set_header(&mut self, name: &str, value: &str) -> &mut Self {
        (*self).set_header(name, value);
        self
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

pub trait SendStatus {
    fn set_ok(&mut self) -> &mut Self {
        self.set_status(200)
    }

    fn set_status(&mut self, status: u16) -> &mut Self;

    fn set_status_message(&mut self, message: &str) -> &mut Self;

    fn status(mut self, status: u16) -> Self
    where
        Self: Sized,
    {
        self.set_status(status);
        self
    }

    fn status_message<M>(mut self, message: M) -> Self
    where
        M: AsRef<str>,
        Self: Sized,
    {
        self.set_status_message(message.as_ref());
        self
    }
}

impl<S> SendStatus for &mut S
where
    S: SendStatus,
{
    fn set_status(&mut self, status: u16) -> &mut Self {
        (*self).set_status(status);
        self
    }

    fn set_status_message(&mut self, message: &str) -> &mut Self {
        (*self).set_status_message(message);
        self
    }
}

pub trait Query {
    fn query(&self) -> &'_ str;
}

impl<Q> Query for &Q
where
    Q: Query,
{
    fn query(&self) -> &'_ str {
        (*self).query()
    }
}

pub trait RequestId {
    fn get_request_id(&self) -> &'_ str;
}

impl<R> RequestId for &R
where
    R: RequestId,
{
    fn get_request_id(&self) -> &'_ str {
        (*self).get_request_id()
    }
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use crate::unblocker::asynch::Blocking;

    impl<B, Q> super::Query for Blocking<B, Q>
    where
        Q: super::Query,
    {
        fn query(&self) -> &'_ str {
            self.1.query()
        }
    }

    impl<B, R> super::RequestId for Blocking<B, R>
    where
        R: super::RequestId,
    {
        fn get_request_id(&self) -> &'_ str {
            self.1.get_request_id()
        }
    }

    impl<B, H> super::Headers for Blocking<B, H>
    where
        H: super::Headers,
    {
        fn header(&self, name: &str) -> Option<&'_ str> {
            self.1.header(name)
        }
    }

    impl<B, S> super::Status for Blocking<B, S>
    where
        S: super::Status,
    {
        fn status(&self) -> u16 {
            self.1.status()
        }

        fn status_message(&self) -> Option<&'_ str> {
            self.1.status_message()
        }
    }

    impl<B, S> super::SendStatus for Blocking<B, S>
    where
        S: super::SendStatus,
    {
        fn set_status(&mut self, status: u16) -> &mut Self {
            self.1.set_status(status);
            self
        }

        fn set_status_message(&mut self, message: &str) -> &mut Self {
            self.1.set_status_message(message);
            self
        }
    }

    impl<B, S> super::SendHeaders for Blocking<B, S>
    where
        S: super::SendHeaders,
    {
        fn set_header(&mut self, name: &str, value: &str) -> &mut Self {
            self.1.set_header(name, value);
            self
        }
    }
}

pub mod cookies {
    use core::iter::{self, Iterator};
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
