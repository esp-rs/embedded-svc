use core::fmt::Write as _;

use embedded_io::blocking::Write;

use crate::http::server::{Connection, Handler, HandlerResult, Middleware, Request};
use crate::http::{headers, Headers};
use crate::utils::mutex::{Mutex, RawMutex};

pub struct WithCaptivePortalMiddleware<M, F>
where
    M: RawMutex,
{
    captive: Mutex<M, bool>,
    portal_uri: &'static str,
    allowed_hosts: F,
}

impl<M, F> WithCaptivePortalMiddleware<M, F>
where
    M: RawMutex,
    F: Fn(&str) -> bool,
{
    pub fn new(captive: bool, portal_uri: &'static str, allowed_hosts: F) -> Self {
        Self {
            captive: Mutex::new(captive),
            portal_uri,
            allowed_hosts,
        }
    }
}

impl<C, M, F> Middleware<C> for WithCaptivePortalMiddleware<M, F>
where
    C: Connection,
    M: RawMutex + Send + Sync,
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
    captive: &Mutex<M, bool>,
) -> HandlerResult
where
    C: Connection,
    M: RawMutex,
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
