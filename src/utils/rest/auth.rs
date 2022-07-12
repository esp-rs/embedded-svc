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
    fn handle<H>(&self, connection: &mut C, mut request: C::Request, handler: &H) -> HandlerResult
    where
        H: Handler<C>,
    {
        let headers = connection.headers(&mut request);

        let role = (self.auth)(headers);

        if let Some(role) = role {
            if role >= self.min_role {
                return handler.handle(connection, request);
            }
        } else {
            let role = self
                .session
                .as_ref()
                .and_then(|session| {
                    session.with_existing(get_cookie_session_id(headers), |sd| sd.get_role())
                })
                .flatten();

            if let Some(role) = role {
                if role >= self.min_role {
                    return handler.handle(connection, request);
                }
            }
        }

        connection.into_response(
            request,
            401,
            None,
            &[("WWW-Authenticate", "Basic realm=\"User Visible Realm\"")],
        )?;

        Ok(())
    }
}

pub fn relogin<C: Connection>(
    connection: &mut C,
    mut request: C::Request,
    session: &impl Session<SessionData = impl RoleSessionData>,
    auth: impl Fn(&str, &str) -> Option<Role>,
) -> HandlerResult {
    if session
        .with_existing(
            get_cookie_session_id(connection.headers(&mut request)),
            |sd| sd.get_role(),
        )
        .flatten()
        .is_some()
    {
        login(connection, request, session, auth)?;
    }

    Ok(())
}

pub fn login<C: Connection>(
    connection: &mut C,
    mut request: C::Request,
    session: &impl Session<SessionData = impl RoleSessionData>,
    auth: impl Fn(&str, &str) -> Option<Role>,
) -> HandlerResult {
    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct Credentials {
        username: heapless::String<32>,
        password: heapless::String<32>,
    }

    let credentials: Credentials = json_io::read::<512, _, _>(connection.reader(&mut request))?;

    if let Some(role) = auth(&credentials.username, &credentials.password) {
        let headers = connection.headers(&mut request);

        session.invalidate(get_cookie_session_id(headers));

        let session_id = "XXX"; // TODO: Random string
        session.with(session_id, |sd| sd.set_role(role))?;

        let mut cookie = heapless::String::<128>::new();
        set_cookie_session_id(headers, session_id, &mut cookie);

        connection.into_response(request, 200, None, &[("Set-Cookie", cookie.as_str())])?;

        Ok(())
    } else {
        let mut response = connection.into_status_response(request, 401)?;

        Ok(connection
            .writer(&mut response)
            .write_all("Invalid username or password".as_bytes())?)
    }
}

pub fn logout<C: Connection>(
    connection: &mut C,
    mut request: C::Request,
    session: &impl Session,
) -> HandlerResult {
    session.invalidate(get_cookie_session_id(connection.headers(&mut request)));

    Ok(())
}
