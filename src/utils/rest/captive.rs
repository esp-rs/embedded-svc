use core::fmt;

extern crate alloc;
use alloc::borrow::ToOwned;
use alloc::sync::Arc;

use anyhow::Result;

use crate::{
    http::server::*,
    http::{
        server::{middleware::Middleware, registry::*},
        Headers, SendHeaders, SendStatus,
    },
    mutex::*,
};

pub fn register<R, M>(
    registry: &mut R,
    pref: impl AsRef<str>,
    portal_uri: impl AsRef<str> + 'static,
    captive: Arc<M>,
) -> Result<()>
where
    R: Registry,
    M: Mutex<Data = bool> + 'static,
{
    let pref = pref.as_ref();

    let prefix = |s| [pref, s].concat();

    registry
        .at(prefix(""))
        .get(move |req| get_status(req, portal_uri.as_ref(), &*captive))
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

fn get_status<'a, M>(
    _req: &mut impl Request<'a>,
    portal_uri: impl AsRef<str>,
    captive: &M,
) -> Result<ResponseData>
where
    M: Mutex<Data = bool>,
{
    let data = format!(
        r#"
        {{
            "captive": {},
            "user-portal-url": "{}"
        }}"#,
        *captive.lock(),
        portal_uri.as_ref(),
    );

    ResponseData::ok()
        .content_type("application/captive+json")
        .body(data.into())
        .into()
}

#[derive(Clone)]
pub struct WithCaptivePortalMiddleware<M, F: Clone> {
    pub portal_uri: &'static str,
    pub captive: Arc<M>,
    pub allowed_hosts: Option<F>,
}

impl<M, F, R> Middleware<R> for WithCaptivePortalMiddleware<M, F>
where
    M: Mutex<Data = bool>,
    F: Fn(&str) -> bool + Clone,
    R: Registry,
{
    type Error = anyhow::Error;

    fn handle<'a, H, E>(
        &self,
        req: R::Request<'a>,
        resp: R::Response<'a>,
        handler: H,
    ) -> Result<Completion, Self::Error>
    where
        R: Registry,
        H: FnOnce(R::Request<'a>, R::Response<'a>) -> Result<Completion, E>,
        E: fmt::Display + fmt::Debug,
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
            handler(req, resp).map_err(|e| anyhow::format_err!("ERROR {}", e))
        } else {
            let completion = resp
                .status(307)
                .header("Location", self.portal_uri.to_owned())
                .submit(req)
                .map_err(|e| anyhow::anyhow!(e))?;

            Ok(completion)
        }
    }
}
