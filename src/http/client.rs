use core::cmp::min;

use crate::io::{self, Write};

use super::{HttpHeaders, HttpMethod, HttpSendHeaders, HttpStatus};

pub trait HttpClient {
    type Request<'a>: HttpRequest<'a, Error = Self::Error>
    where
        Self: 'a;

    #[cfg(not(feature = "std"))]
    type Error;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;

    fn get(&mut self, url: impl AsRef<str>) -> Result<Self::Request<'_>, Self::Error> {
        self.request(HttpMethod::Get, url)
    }

    fn post(&mut self, url: impl AsRef<str>) -> Result<Self::Request<'_>, Self::Error> {
        self.request(HttpMethod::Post, url)
    }

    fn request(
        &mut self,
        method: HttpMethod,
        url: impl AsRef<str>,
    ) -> Result<Self::Request<'_>, Self::Error>;
}

pub trait HttpRequest<'a>: HttpSendHeaders<'a> {
    type Response<'b>: HttpResponse<'b, Error = Self::Error>;
    type Write<'b>: io::Write<Error = Self::Error>;

    #[cfg(not(feature = "std"))]
    type Error;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;

    fn send_bytes(self, bytes: impl AsRef<[u8]>) -> Result<Self::Response<'a>, Self::Error>
    where
        Self: Sized,
    {
        self.send(bytes.as_ref().len(), |write| {
            write.do_write_all(bytes.as_ref())
        })
    }

    fn send_str(self, s: impl AsRef<str>) -> Result<Self::Response<'a>, Self::Error>
    where
        Self: Sized,
    {
        self.send_bytes(s.as_ref().as_bytes())
    }

    fn send_json<T>(self, _t: impl AsRef<T>) -> Result<Self::Response<'a>, Self::Error>
    where
        Self: Sized,
    {
        todo!()
    }

    fn send_reader<R: io::Read<Error = Self::Error>>(
        self,
        size: usize,
        mut read: R,
    ) -> Result<Self::Response<'a>, Self::Error>
    where
        Self: Sized,
    {
        self.send(size, |write| {
            let mut size = size;

            let mut buf = [0; 128];
            let buf_len = buf.len();

            while size > 0 {
                let s = read.do_read(&mut buf[0..min(size, buf_len)])?;
                write.do_write_all(&buf[0..s])?;

                size -= s;
            }

            Ok(())
        })
    }

    fn send(
        self,
        size: usize,
        f: impl FnOnce(&mut Self::Write<'a>) -> Result<(), Self::Error>,
    ) -> Result<Self::Response<'a>, Self::Error>;

    fn submit(self) -> Result<Self::Response<'a>, Self::Error>
    where
        Self: Sized,
    {
        self.send_bytes(&[0_u8; 0])
    }
}

pub trait HttpResponse<'a>: HttpStatus + HttpHeaders {
    type Read<'b>: io::Read<Error = Self::Error>;

    #[cfg(not(feature = "std"))]
    type Error;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;

    fn payload<'b>(&'b mut self) -> &mut Self::Read<'b>;

    fn into_payload(self) -> Self::Read<'a>;
}
