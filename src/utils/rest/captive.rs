use core::fmt::Write as _;
use core::iter;

use embedded_io::blocking::Write;

use crate::http::headers;
use crate::http::server::{Handler, HandlerResult, Middleware, Request};
use crate::mutex::*;

pub struct WithCaptivePortalMiddleware<M, F> {
    captive: M,
    portal_uri: &'static str,
    allowed_hosts: F,
}

impl<M, F> WithCaptivePortalMiddleware<M, F>
where
    M: Mutex<Data = bool>,
    F: Fn(&str) -> bool,
{
    pub fn new(captive: bool, portal_uri: &'static str, allowed_hosts: F) -> Self {
        Self {
            captive: M::new(captive),
            portal_uri,
            allowed_hosts,
        }
    }
}

impl<R, M, F> Middleware<R> for WithCaptivePortalMiddleware<M, F>
where
    R: Request,
    M: Mutex<Data = bool> + Send,
    F: Fn(&str) -> bool + Send,
{
    fn handle<H>(&self, request: R, handler: &H) -> HandlerResult
    where
        H: Handler<R>,
    {
        let captive = *self.captive.lock();

        let allow = !captive
            || request
                .header("host")
                .map(|host| (self.allowed_hosts)(host))
                .unwrap_or(true);

        if allow {
            handler.handle(request)
        } else {
            request.into_response(307, None, iter::once(("Location", self.portal_uri)))?;

            Ok(())
        }
    }
}

pub fn get_status<R, M, const N: usize>(request: R, portal_uri: &str, captive: &M) -> HandlerResult
where
    R: Request,
    M: Mutex<Data = bool>,
{
    let mut data = heapless::String::<N>::new();

    write!(
        &mut data,
        r#"
        {{
            "captive": {},
            "user-portal-url": "{}"
        }}"#,
        *captive.lock(),
        portal_uri,
    )
    .unwrap();

    Ok(request
        .into_response(200, None, headers::content_type("application/captive+json"))?
        .write_all(data.as_bytes())?)
}
