use core::fmt::Write;

use crate::http::server::middleware::Middleware;
use crate::http::server::registry::Registry;
use crate::http::server::{Completion, Response};
use crate::http::server::{Handler, HandlerError, Request};
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

impl<R, P, M, F> Middleware<R, P> for WithCaptivePortalMiddleware<M, F>
where
    R: Request,
    P: Response,
    M: Mutex<Data = bool> + Send,
    F: Fn(&str) -> bool + Send,
{
    fn handle<H>(&self, req: R, resp: P, handler: &H) -> Result<Completion, HandlerError>
    where
        H: Handler<R, P>,
    {
        let captive = *self.captive.lock();

        let allow = !captive
            || req
                .header("host")
                .map(|host| (self.allowed_hosts)(host))
                .unwrap_or(true);

        if allow {
            handler.handle(req, resp)
        } else {
            let completion = resp
                .status(307)
                .header("Location", self.portal_uri)
                .submit()?;

            Ok(completion)
        }
    }
}

pub fn register<R, M, const N: usize>(
    registry: &mut R,
    portal_uri: &'static str,
    captive: M,
) -> Result<(), R::Error>
where
    R: Registry,
    M: Mutex<Data = bool> + Send + Sync + 'static,
{
    registry.handle_get("", move |req, resp| {
        get_status::<_, _, _, N>(req, resp, portal_uri, &captive)
    })?;

    Ok(())
}

pub fn get_status<R, P, M, const N: usize>(
    _req: R,
    resp: P,
    portal_uri: &str,
    captive: &M,
) -> Result<Completion, HandlerError>
where
    R: Request,
    P: Response,
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

    Ok(resp
        .content_type("application/captive+json")
        .send_str(&data)?)
}
