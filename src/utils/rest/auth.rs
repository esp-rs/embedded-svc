use core::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::errors::wrap::{EitherError, EitherError4};
use crate::http::server::session::Session;
use crate::http::server::*;
use crate::io::read_max;

use crate::utils::role::*;

pub trait RoleSessionData {
    fn get_role(&self) -> Option<Role>;
    fn set_role(&mut self, role: Role);
}

pub fn with_role<R, S, E>(
    req: R,
    resp: S,
    handler: impl Fn(R, S, Role) -> Result<Completion, E>,
    min_role: Role,
    auth: impl Fn(&R) -> Option<Role>,
    session: &impl Session<SessionData = impl RoleSessionData>,
) -> Result<Completion, impl Debug>
where
    R: Request,
    S: Response,
    E: Debug,
{
    let role = session
        .with_existing(&req, |sd| sd.get_role())
        .flatten()
        .or_else(|| auth(&req));

    if let Some(role) = role {
        if role >= min_role {
            return handler(req, resp, role).map_err(EitherError::E2);
        }
    }

    let completion = resp
        .status(401)
        .header("WWW-Authenticate", "Basic realm=\"User Visible Realm\"")
        .submit(req)
        .map_err(EitherError::E1)?;

    Ok(completion)
}

pub fn login(
    mut req: impl Request,
    mut resp: impl Response,
    session: &impl Session<SessionData = impl RoleSessionData>,
    auth: impl Fn(&str, &str) -> Option<Role>,
) -> Result<Completion, impl Debug> {
    if session
        .with_existing(&req, |sd| sd.get_role())
        .flatten()
        .is_some()
    {
        resp.submit(req).map_err(EitherError4::E1)
    } else {
        let mut buf = [0_u8; 1000];

        let (buf, _) = read_max(req.reader(), &mut buf).map_err(EitherError4::E2)?;

        #[derive(Clone, Debug, Serialize, Deserialize)]
        struct Credentials<'a> {
            username: &'a str,
            password: &'a str,
        }

        let credentials: Credentials = serde_json::from_slice(&buf).map_err(EitherError4::E3)?;

        if let Some(role) = auth(credentials.username, credentials.password) {
            session.invalidate(&req);

            session
                .with(&req, &mut resp, |sd| sd.set_role(role))
                .map_err(EitherError4::E4)?;

            resp.submit(req).map_err(EitherError4::E1)
        } else {
            resp.status(401)
                .send_str(req, "Invalid username or password")
                .map_err(EitherError4::E1)
        }
    }
}

pub fn logout(
    req: impl Request,
    resp: impl Response,
    session: impl Session,
) -> Result<Completion, impl Debug> {
    session.invalidate(&req);

    resp.submit(req)
}
