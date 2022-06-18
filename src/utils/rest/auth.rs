use core::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::http::server::middleware::Middleware;
use crate::http::server::registry::Registry;
use crate::http::server::session::Session;
use crate::http::server::*;
use crate::io::read_max;

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
    fn handle<H>(&self, req: R, resp: P, handler: &H) -> Result<Completion, HandlerError>
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

        let completion = resp
            .status(401)
            .header("WWW-Authenticate", "Basic realm=\"User Visible Realm\"")
            .submit()?;

        Ok(completion)
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
) -> Result<Completion, HandlerError> {
    if session
        .with_existing(&req, |sd| sd.get_role())
        .flatten()
        .is_some()
    {
        Ok(resp.submit()?)
    } else {
        let mut buf = [0_u8; 1000];

        let (buf, _) = read_max(req.reader(), &mut buf)?;

        #[derive(Clone, Debug, Serialize, Deserialize)]
        struct Credentials<'a> {
            username: &'a str,
            password: &'a str,
        }

        let credentials: Credentials = serde_json::from_slice(&buf)?;

        if let Some(role) = auth(credentials.username, credentials.password) {
            session.invalidate(&req);

            session.with(&req, &mut resp, |sd| sd.set_role(role))?;

            Ok(resp.submit()?)
        } else {
            Ok(resp.status(401).send_str("Invalid username or password")?)
        }
    }
}

pub fn logout(
    req: impl Request,
    resp: impl Response,
    session: &impl Session,
) -> Result<Completion, HandlerError> {
    session.invalidate(&req);

    Ok(resp.submit()?)
}
