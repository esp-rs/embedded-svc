use core::fmt::Write as _;

use embedded_io::blocking::Write;

use crate::http::server::{Connection, Handler, HandlerResult, Middleware, Request};
use crate::http::{headers, Headers};
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

impl<C, M, F> Middleware<C> for WithCaptivePortalMiddleware<M, F>
where
    C: Connection,
    M: Mutex<Data = bool> + Send,
    F: Fn(&str) -> bool + Send,
{
    fn handle<H>(&self, connection: C, handler: &H) -> HandlerResult
    where
        H: Handler<C>,
    {
        let request = Request::wrap(connection)?;

        let captive = *self.captive.lock();

        let allow = !captive
            || request
                .header("host")
                .map(|host| (self.allowed_hosts)(host))
                .unwrap_or(true);

        if allow {
            handler.handle(request.release())
        } else {
            request.into_response(307, None, &[headers::location(self.portal_uri)])?;

            Ok(())
        }
    }
}

pub fn get_status<C, M, const N: usize>(
    request: Request<C>,
    portal_uri: &str,
    captive: &M,
) -> HandlerResult
where
    C: Connection,
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

    let mut response = request.into_response(
        200,
        None,
        &[headers::content_type("application/captive+json")],
    )?;

    Ok(response.write_all(data.as_bytes())?)
}
