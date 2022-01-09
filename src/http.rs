extern crate alloc;
use alloc::borrow::Cow;
use alloc::string::ToString;

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

pub trait SendHeaders<'a> {
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

pub trait Status {
    fn status(&self) -> u16;
    fn status_message(&self) -> Option<Cow<'_, str>>;
}

pub trait SendStatus<'a> {
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

pub mod cookies {
    use core::iter::{FromIterator, Iterator};
    use core::str::Split;

    extern crate alloc;
    use alloc::borrow::Cow;
    use alloc::string::String;

    pub struct Cookies<'a>(Cow<'a, str>);

    impl<'a> Cookies<'a> {
        pub fn new(cookies_str: impl Into<Cow<'a, str>>) -> Self {
            Self(cookies_str.into())
        }

        pub fn get(&self, name: impl AsRef<str>) -> Option<&'_ str> {
            let name = name.as_ref();

            self.into_iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value)
        }

        pub fn insert(&self, name: impl AsRef<str>, value: impl AsRef<str>) -> Cookies<'static> {
            let name = name.as_ref();

            self.into_iter()
                .chain(core::iter::once((name, value.as_ref())))
                .filter(|(key, _)| *key != name)
                .collect()
        }

        pub fn remove(&self, name: impl AsRef<str>) -> Cookies<'static> {
            let name = name.as_ref();

            self.into_iter().filter(|(key, _)| *key != name).collect()
        }
    }

    impl<'a> AsRef<str> for Cookies<'a> {
        fn as_ref(&self) -> &str {
            self.0.as_ref()
        }
    }

    impl<'a> From<Cookies<'a>> for Cow<'a, str> {
        fn from(cookies: Cookies<'a>) -> Self {
            cookies.0
        }
    }

    impl<'a, 'b> IntoIterator for &'b Cookies<'a>
    where
        'a: 'b,
    {
        type Item = (&'b str, &'b str);

        type IntoIter = CookieIterator<'b>;

        fn into_iter(self) -> Self::IntoIter {
            CookieIterator::new(self.0.as_ref())
        }
    }

    impl<'a> FromIterator<(&'a str, &'a str)> for Cookies<'static> {
        fn from_iter<T: IntoIterator<Item = (&'a str, &'a str)>>(iter: T) -> Self {
            let mut result = String::new();
            for (key, value) in iter {
                if !result.is_empty() {
                    result.push(';');
                }

                result.push_str(key);
                result.push('=');
                result.push_str(value);
            }

            Self(Cow::Owned(result))
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
