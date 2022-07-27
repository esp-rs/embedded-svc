use core::fmt::Debug;

use embedded_io::blocking::Write;
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

impl<C, A, S, D> Middleware<C> for WithRoleMiddleware<A, S>
where
    C: Connection,
    A: Fn(&C::Headers) -> Option<Role> + Send,
    S: Session<SessionData = D>,
    D: RoleSessionData,
{
    fn handle<H>(&self, connection: &mut C, handler: &H) -> HandlerResult
    where
        H: Handler<C>,
    {
        let role = (self.auth)(connection.headers()?);

        let request = Request::wrap(connection)?;

        if let Some(role) = role {
            if role >= self.min_role {
                return handler.handle(connection);
            }
        } else {
            let role = self
                .session
                .as_ref()
                .and_then(|session| {
                    session.with_existing(get_cookie_session_id(&request), |sd| sd.get_role())
                })
                .flatten();

            if let Some(role) = role {
                if role >= self.min_role {
                    return handler.handle(connection);
                }
            }
        }

        request.into_response(
            401,
            None,
            &[("WWW-Authenticate", "Basic realm=\"User Visible Realm\"")],
        )?;

        Ok(())
    }
}

pub fn relogin<'a, C: Connection>(
    request: Request<'a, C>,
    session: &impl Session<SessionData = impl RoleSessionData>,
    auth: impl Fn(&str, &str) -> Option<Role>,
) -> HandlerResult {
    if session
        .with_existing(get_cookie_session_id(&request), |sd| sd.get_role())
        .flatten()
        .is_some()
    {
        login(request, session, auth)?;
    }

    Ok(())
}

pub fn login<'a, C: Connection>(
    mut request: Request<'a, C>,
    session: &impl Session<SessionData = impl RoleSessionData>,
    auth: impl Fn(&str, &str) -> Option<Role>,
) -> HandlerResult {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct Credentials {
        username: heapless::String<32>,
        password: heapless::String<32>,
    }

    let credentials: Credentials = json_io::read::<512, _, _>(&mut request)?;

    if let Some(role) = auth(&credentials.username, &credentials.password) {
        session.invalidate(get_cookie_session_id(&request));

        let session_id = "XXX"; // TODO: Random string
        session.with(session_id, |sd| sd.set_role(role))?;

        let mut cookie = heapless::String::<128>::new();
        set_cookie_session_id(&request, session_id, &mut cookie);

        request.into_response(200, None, &[("Set-Cookie", cookie.as_str())])?;

        Ok(())
    } else {
        let mut response = request.into_status_response(401)?;

        Ok(response.write_all("Invalid username or password".as_bytes())?)
    }
}

pub fn logout<'a, C: Connection>(request: Request<'a, C>, session: &impl Session) -> HandlerResult {
    session.invalidate(get_cookie_session_id(&request));

    Ok(())
}
