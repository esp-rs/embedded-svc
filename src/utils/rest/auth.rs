use core::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::http::server::middleware::Middleware;
use crate::http::server::registry::Registry;
use crate::http::server::*;
use crate::io::Write;

use crate::utils::http::server::session::*;
use crate::utils::json_io;
use crate::utils::role::*;

pub trait RoleSessionData {
    fn get_role(&self) -> Option<Role>;
    fn set_role(&mut self, role: Role);
}

pub struct WithRoleMiddleware<A, S> {
    auth: A,
    session: Option<S>,
    min_role: Role,
}

impl<A, S> WithRoleMiddleware<A, S> {
    pub fn new(auth: A, session: Option<S>, min_role: Role) -> Self {
        Self {
            auth,
            session,
            min_role,
        }
    }
}

impl<R, P, A, S, D> Middleware<R, P> for WithRoleMiddleware<A, S>
where
    R: Request,
    P: Response,
    A: Fn(&R) -> Option<Role> + Send,
    S: Session<SessionData = D>,
    D: RoleSessionData,
{
    fn handle<H>(&self, req: R, resp: P, handler: &H) -> Result<(), HandlerError>
    where
        H: Handler<R, P>,
    {
        let role = (self.auth)(&req);

        if let Some(role) = role {
            if role >= self.min_role {
                return handler.handle(req, resp);
            }
        } else {
            let role = self
                .session
                .as_ref()
                .and_then(|session| session.with_existing(&req, |sd| sd.get_role()))
                .flatten();

            if let Some(role) = role {
                if role >= self.min_role {
                    return handler.handle(req, resp);
                }
            }
        }

        resp.status(401)
            .header("WWW-Authenticate", "Basic realm=\"User Visible Realm\"");

        Ok(())
    }
}

pub fn register<R>(
    registry: &mut R,
    session: impl Session<SessionData = impl RoleSessionData> + Clone + 'static,
    auth: impl Fn(&str, &str) -> Option<Role> + Send + Sync + 'static,
) -> Result<(), R::Error>
where
    R: Registry,
{
    let session1 = session.clone();

    registry
        .handle_post("/login", move |req, resp| {
            login(req, resp, &session1, &auth)
        })?
        .handle_post("/logout", move |req, resp| logout(req, resp, &session))?;

    Ok(())
}

pub fn login(
    mut req: impl Request,
    mut resp: impl Response,
    session: &impl Session<SessionData = impl RoleSessionData>,
    auth: impl Fn(&str, &str) -> Option<Role>,
) -> Result<(), HandlerError> {
    if session
        .with_existing(&req, |sd| sd.get_role())
        .flatten()
        .is_some()
    {
        Ok(())
    } else {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        struct Credentials {
            username: heapless::String<32>,
            password: heapless::String<32>,
        }

        let credentials: Credentials = json_io::read::<512, _, _>(&mut req)?;

        if let Some(role) = auth(&credentials.username, &credentials.password) {
            session.invalidate(&req);

            session.with(&req, &mut resp, |sd| sd.set_role(role))?;

            Ok(())
        } else {
            resp.status(401)
                .into_writer()?
                .write_all("Invalid username or password".as_bytes())?;

            Ok(())
        }
    }
}

pub fn logout(
    req: impl Request,
    _resp: impl Response,
    session: &impl Session,
) -> Result<(), HandlerError> {
    session.invalidate(&req);

    Ok(())
}
