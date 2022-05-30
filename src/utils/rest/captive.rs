use core::fmt::{self, Debug};

use crate::errors::EitherError;
use crate::http::server::Request;
use crate::http::server::{middleware::Middleware, registry::*, Completion, Context, Response};
use crate::http::{Headers, SendHeaders, SendStatus};
use crate::mutex::*;

pub fn register<R, M>(
    registry: &mut R,
    portal_uri: &'static str,
    captive: M,
) -> Result<(), R::Error>
where
    R: Registry,
    M: Mutex<Data = bool> + Send + Sync + 'static,
{
    registry
        .at("")
        .inline()
        .get(move |req, resp| get_status(req, resp, portal_uri, &captive))?;

    Ok(())
}

fn get_status(
    req: impl Request,
    resp: impl Response,
    portal_uri: impl AsRef<str>,
    captive: &impl Mutex<Data = bool>,
) -> Result<Completion, impl Debug> {
    let data = format!(
        r#"
        {{
            "captive": {},
            "user-portal-url": "{}"
        }}"#,
        *captive.lock(),
        portal_uri.as_ref(),
    );

    resp.content_type("application/captive+json")
        .send_str(req, &data)
}

#[derive(Clone)]
pub struct WithCaptivePortalMiddleware<M, F: Clone> {
    pub portal_uri: &'static str,
    pub captive: M,
    pub allowed_hosts: Option<F>,
}

impl<M, F, C> Middleware<C> for WithCaptivePortalMiddleware<M, F>
where
    M: Mutex<Data = bool> + Send + Sync,
    F: Fn(&str) -> bool + Clone + Send,
    C: Context,
{
    type Error = C::Error;

    fn handle<'a, H, E>(
        &self,
        req: C::Request,
        resp: C::Response,
        handler: H,
    ) -> Result<Completion, EitherError<Self::Error, E>>
    where
        H: Fn(C::Request, C::Response) -> Result<Completion, E> + Send + Sync,
        E: fmt::Debug,
    {
        let captive = *self.captive.lock();

        let allow = !captive
            || self
                .allowed_hosts
                .as_ref()
                .and_then(|allowed_hosts| {
                    req.header("host").map(|host| allowed_hosts(host.as_ref()))
                })
                .unwrap_or(true);

        if allow {
            handler(req, resp).map_err(EitherError::Second)
        } else {
            let completion = resp
                .status(307)
                .header("Location", self.portal_uri.to_owned())
                .submit(req)
                .map_err(EitherError::First)?;

            Ok(completion)
        }
    }
}
