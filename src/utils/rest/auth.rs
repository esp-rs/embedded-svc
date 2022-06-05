use core::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::errors::either::{EitherError, EitherError4};
use crate::http::server::middleware::Middleware;
use crate::http::server::registry::*;
use crate::http::server::*;
use crate::http::*;
use crate::io::read_max;

use crate::storage::{DynStorage, Storage};
use crate::utils::role::*;

pub trait Authenticator {
    fn authenticate(&self, username: impl AsRef<str>, password: impl AsRef<str>) -> Option<Role>;
}

#[derive(Clone)]
pub struct WithRoleMiddleware {
    pub role: Role,
    pub default_role: Option<Role>,
}

impl<C> Middleware<C> for WithRoleMiddleware
where
    C: Context,
{
    type Error = C::Error;

    fn handle<H, E>(
        &self,
        mut req: C::Request,
        resp: C::Response,
        handler: H,
    ) -> Result<Completion, EitherError<Self::Error, E>>
    where
        H: FnOnce(C::Request, C::Response) -> Result<Completion, E> + Send,
        E: Debug,
    {
        let current_role = get_role(&mut req, self.default_role).map_err(EitherError::E1)?;

        if let Some(current_role) = current_role {
            if current_role >= self.role {
                return handler(req, resp).map_err(EitherError::E2);
            }
        }

        let completion = resp.status(400).submit(req).map_err(EitherError::E1)?;

        Ok(completion)
    }
}

#[cfg(feature = "std")]
#[derive(Clone)]
pub struct WithBasicAuthMiddleware<A> {
    pub authenticator: A,
    pub min_role: Role,
}

#[cfg(feature = "std")]
impl<A, C> Middleware<C> for WithBasicAuthMiddleware<A>
where
    A: Authenticator + Clone + Send,
    C: Context,
{
    type Error = C::Error;

    fn handle<'a, H, E>(
        &self,
        mut req: C::Request,
        resp: C::Response,
        handler: H,
    ) -> Result<Completion, EitherError<Self::Error, E>>
    where
        H: FnOnce(C::Request, C::Response) -> Result<Completion, E>,
        E: Debug,
    {
        if let Some(role) = get_role(&mut req, None).map_err(EitherError::E1)? {
            if role >= self.min_role {
                return handler(req, resp).map_err(EitherError::E2);
            }
        }

        let authorization = req.header("Authorization");
        if let Some(authorization) = authorization {
            if let Ok(credentials) =
                http_auth_basic::Credentials::from_header(authorization.to_owned())
            {
                if let Some(role) = self
                    .authenticator
                    .authenticate(credentials.user_id, credentials.password)
                {
                    if role >= self.min_role {
                        set_request_role(&mut req, Some(Role::Admin)).map_err(EitherError::E1)?;

                        return handler(req, resp).map_err(EitherError::E2);
                    }
                }
            }
        }

        let completion = resp
            .status(401)
            .header("WWW-Authenticate", "Basic realm=\"User Visible Realm\"")
            .submit(req)
            .map_err(EitherError::E1)?;

        Ok(completion)
    }
}

#[derive(Clone)]
pub struct WithSessionAuthMiddleware<'l> {
    pub login: &'l str,
    pub min_role: Role,
}

impl<'l, C> Middleware<C> for WithSessionAuthMiddleware<'l>
where
    C: Context,
{
    type Error = C::Error;

    fn handle<'a, H, E>(
        &self,
        mut req: C::Request,
        resp: C::Response,
        handler: H,
    ) -> Result<Completion, EitherError<Self::Error, E>>
    where
        H: FnOnce(C::Request, C::Response) -> Result<Completion, E>,
        E: Debug,
    {
        if let Some(role) = get_role(&mut req, None).map_err(EitherError::E1)? {
            if role >= self.min_role {
                return handler(req, resp).map_err(EitherError::E2);
            }
        }

        let completion = resp
            .redirect(req, self.login.as_ref())
            .map_err(EitherError::E1)?;

        Ok(completion)
    }
}

pub fn get_role<R>(req: &mut R, default_role: Option<Role>) -> Result<Option<Role>, R::Error>
where
    R: Request,
{
    let role = if let Some(role) = req.attrs().get("role")? {
        role.downcast_ref::<Role>().cloned()
    } else if let Some(role) = req.session().get("role").ok().flatten() {
        Some(role)
    } else {
        default_role
    };

    Ok(role)
}

pub fn set_request_role<R>(req: &mut R, role: Option<Role>) -> Result<(), R::Error>
where
    R: Request,
{
    if let Some(role) = role {
        req.attrs().set("role", &role)?;
    } else {
        req.attrs().remove("role")?;
    }

    Ok(())
}

pub fn set_session_role(req: &mut impl Request, role: Option<Role>) -> Result<(), SessionError> {
    if let Some(role) = role {
        req.session().set("role", &role)?;
    } else {
        req.session().remove("role")?;
    }

    Ok(())
}

pub fn register<R, A>(registry: &mut R, authenticator: A) -> Result<(), R::Error>
where
    R: Registry,
    A: Authenticator + Send + Sync + 'static,
{
    registry
        .at("/login")
        .inline()
        .post(move |req, resp| login(req, resp, &authenticator))?
        .at("/logout")
        .inline()
        .post(move |req, resp| logout(req, resp))?;

    Ok(())
}

fn login(
    mut req: impl Request,
    resp: impl Response,
    authenticator: &impl Authenticator,
) -> Result<Completion, impl Debug> {
    if req.session().is_valid() {
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

        if let Some(role) = authenticator.authenticate(credentials.username, credentials.password) {
            {
                let mut session = req.session();

                session.invalidate().map_err(EitherError4::E4)?;

                session.create_if_invalid().map_err(EitherError4::E4)?;
            }

            set_session_role(&mut req, Some(role)).map_err(EitherError4::E4)?;

            resp.submit(req).map_err(EitherError4::E1)
        } else {
            resp.status(401)
                .send_str(req, "Invalid username or password")
                .map_err(EitherError4::E1)
        }
    }
}

fn logout(req: impl Request, resp: impl Response) -> Result<Completion, impl Debug> {
    req.session().invalidate().map_err(EitherError::E1)?;

    resp.submit(req).map_err(EitherError::E2)
}
