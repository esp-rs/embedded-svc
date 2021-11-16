extern crate alloc;
use alloc::sync::Arc;

use crate::{httpd::registry::*, httpd::*, mutex::*};

pub fn register<R, M>(
    registry: R,
    pref: impl AsRef<str>,
    portal_uri: impl AsRef<str> + 'static,
    captive: Arc<M>,
) -> Result<R>
where
    R: Registry,
    M: Mutex<Data = bool> + 'static,
{
    let pref = pref.as_ref();

    let prefix = |s| [pref, s].concat();

    registry
        .at(prefix(""))
        .get(move |req| get_status(req, portal_uri.as_ref(), &*captive))
}

fn get_status<M>(_req: Request, portal_uri: impl AsRef<str>, captive: &M) -> Result<Response>
where
    M: Mutex<Data = bool>,
{
    let data = format!(
        r#"
        {{
            "captive": {},
            "user-portal-url": "{}"
        }}"#,
        captive.with_lock(|captive| *captive),
        portal_uri.as_ref(),
    );

    Response::ok()
        .content_type("application/captive+json")
        .body(data.into())
        .into()
}

pub fn with_captive<M, H>(
    portal_uri: impl AsRef<str> + 'static,
    captive: Arc<M>,
    allowed_hosts: Option<H>,
) -> impl for<'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response>
where
    M: Mutex<Data = bool>,
    H: Fn(&str) -> bool,
{
    move |req, handler| {
        let captive = captive.with_lock(|captive| *captive);

        let allow = !captive
            || allowed_hosts
                .as_ref()
                .and_then(|allowed_hosts| {
                    req.header("host").map(|host| allowed_hosts(host.as_ref()))
                })
                .unwrap_or(true);

        if allow {
            handler(req)
        } else {
            Response::new(307)
                .header("Location", portal_uri.as_ref().to_owned())
                .into()
        }
    }
}
