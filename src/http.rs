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

pub mod headers {
    use core::iter;

    pub type ContentLenParseBuf = heapless::String<20>;

    pub fn content_type<'a>(ctype: &'a str) -> impl Iterator<Item = (&'static str, &'a str)> {
        iter::once(("Content-Type", ctype))
    }

    pub fn content_len<'a>(
        len: u64,
        buf: &'a mut ContentLenParseBuf,
    ) -> impl Iterator<Item = (&'static str, &'a str)> {
        *buf = ContentLenParseBuf::from(len);

        iter::once(("Content-Length", buf.as_str()))
    }

    pub fn content_encoding<'a>(
        encoding: &'a str,
    ) -> impl Iterator<Item = (&'static str, &'a str)> {
        iter::once(("Content-Encoding", encoding))
    }

    pub fn transfer_encoding<'a>(
        encoding: &'a str,
    ) -> impl Iterator<Item = (&'static str, &'a str)> {
        iter::once(("Transfer-Encoding", encoding))
    }

    pub fn connection<'a>(connection: &'a str) -> impl Iterator<Item = (&'static str, &'a str)> {
        iter::once(("Connection", connection))
    }

    pub fn location<'a>(location: &'a str) -> impl Iterator<Item = (&'static str, &'a str)> {
        iter::once(("Location", location))
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
