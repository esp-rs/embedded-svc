use core::fmt::Debug;

use crate::errors::wrap::EitherError;
use crate::http::server::Request;
use crate::http::server::{Completion, Response};
use crate::mutex::*;

pub fn get_status(
    req: impl Request,
    resp: impl Response,
    portal_uri: &str,
    captive: &impl Mutex<Data = bool>,
) -> Result<Completion, impl Debug> {
    let data = format!(
        // TODO
        r#"
        {{
            "captive": {},
            "user-portal-url": "{}"
        }}"#,
        *captive.lock(),
        portal_uri,
    );

    resp.content_type("application/captive+json")
        .send_str(req, &data)
}

pub fn with_captive_portal<R, S, H, E>(
    req: R,
    resp: S,
    handler: H,
    portal_uri: &str,
    captive: &impl Mutex<Data = bool>,
    allowed_hosts: Option<impl Fn(&str) -> bool>,
) -> Result<Completion, impl Debug>
where
    R: Request,
    S: Response,
    H: Fn(R, S) -> Result<Completion, E>,
    E: Debug,
{
    let captive = *captive.lock();

    let allow = !captive
        || allowed_hosts
            .as_ref()
            .and_then(|allowed_hosts| req.header("host").map(|host| allowed_hosts(host.as_ref())))
            .unwrap_or(true);

    if allow {
        handler(req, resp).map_err(EitherError::E2)
    } else {
        let completion = resp
            .status(307)
            .header("Location", portal_uri.to_owned())
            .submit(req)
            .map_err(EitherError::E1)?;

        Ok(completion)
    }
}
