use core::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::http::server::*;

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

impl<R, A, S, D> Middleware<R> for WithRoleMiddleware<A, S>
where
    R: Request,
    A: Fn(&R) -> Option<Role> + Send,
    S: Session<SessionData = D>,
    D: RoleSessionData,
{
    fn handle<H>(&self, request: R, handler: &H) -> HandlerResult
    where
        H: Handler<R>,
    {
        let role = (self.auth)(&request);

        if let Some(role) = role {
            if role >= self.min_role {
                return handler.handle(request);
            }
        } else {
            let role = self
                .session
                .as_ref()
                .and_then(|session| session.with_existing(&request, |sd| sd.get_role()))
                .flatten();

            if let Some(role) = role {
                if role >= self.min_role {
                    return handler.handle(request);
                }
            }
        }

        Ok(request
            .into_response()?
            .status(401)
            .header("WWW-Authenticate", "Basic realm=\"User Visible Realm\"")
            .complete()?)
    }
}

pub fn login(
    mut request: impl Request,
    session: &impl Session<SessionData = impl RoleSessionData>,
    auth: impl Fn(&str, &str) -> Option<Role>,
) -> HandlerResult {
    if session
        .with_existing(&request, |sd| sd.get_role())
        .flatten()
        .is_some()
    {
        Ok(request.complete()?)
    } else {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        struct Credentials {
            username: heapless::String<32>,
            password: heapless::String<32>,
        }

        let credentials: Credentials = json_io::read::<512, _, _>(&mut request)?;

        if let Some(role) = auth(&credentials.username, &credentials.password) {
            session.invalidate(&request);

            {
                let (headers, _, mut resp_headers) = request.split();
                session.with(&headers, &mut resp_headers, |sd| sd.set_role(role))?;
            }

            Ok(request.into_response()?.complete()?)
        } else {
            Ok(request
                .into_response()?
                .status(401)
                .submit("Invalid username or password".as_bytes())?)
        }
    }
}

pub fn logout(request: impl Request, session: &impl Session) -> HandlerResult {
    session.invalidate(&request);

    Ok(request.complete()?)
}
