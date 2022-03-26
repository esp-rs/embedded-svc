extern crate alloc;
use alloc::borrow::{Cow, ToOwned};
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use anyhow::Result;

use crate::{
    http::server::middleware::Middleware, http::server::registry::*, http::server::*, http::*, io,
};

use crate::utils::role::*;

pub trait Authenticator {
    fn authenticate(&self, username: impl AsRef<str>, password: impl AsRef<str>) -> Option<Role>;
}

#[derive(Clone)]
pub struct WithRoleMiddleware {
    pub role: Role,
    pub default_role: Option<Role>,
}

impl<R> Middleware<R> for WithRoleMiddleware
where
    R: Registry,
{
    type Error = anyhow::Error;

    fn handle<'a, H, E>(
        &self,
        mut req: R::Request<'a>,
        resp: R::Response<'a>,
        handler: H,
    ) -> Result<Completion, Self::Error>
    where
        R: Registry,
        H: FnOnce(R::Request<'a>, R::Response<'a>) -> Result<Completion, E>,
        E: HandlerError,
    {
        let current_role = get_role(&mut req, self.default_role);

        if let Some(current_role) = current_role {
            if current_role >= self.role {
                return handler(req, resp).map_err(|e| anyhow::format_err!("ERROR {}", e));
            }
        }

        let completion = resp
            .status(400)
            .submit(req)
            .map_err(|e| anyhow::anyhow!(e))?;

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
impl<A, R> Middleware<R> for WithBasicAuthMiddleware<A>
where
    A: Authenticator + Clone,
    R: Registry,
{
    type Error = anyhow::Error;

    fn handle<'a, H, E>(
        &self,
        mut req: R::Request<'a>,
        resp: R::Response<'a>,
        handler: H,
    ) -> Result<Completion, Self::Error>
    where
        R: Registry,
        H: FnOnce(R::Request<'a>, R::Response<'a>) -> Result<Completion, E>,
        E: HandlerError,
    {
        if let Some(role) = get_role(&mut req, None) {
            if role >= self.min_role {
                return handler(req, resp).map_err(|e| anyhow::format_err!("ERROR {}", e));
            }
        }

        let authorization = req.header("Authorization");
        if let Some(authorization) = authorization {
            if let Ok(credentials) =
                http_auth_basic::Credentials::from_header(authorization.into_owned())
            {
                if let Some(role) = self
                    .authenticator
                    .authenticate(credentials.user_id, credentials.password)
                {
                    if role >= self.min_role {
                        set_request_role(&mut req, Some(Role::Admin));

                        return handler(req, resp).map_err(|e| anyhow::format_err!("ERROR {}", e));
                    }
                }
            }
        }

        let completion = resp
            .status(401)
            .header("WWW-Authenticate", "Basic realm=\"User Visible Realm\"")
            .submit(req)
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(completion)
    }
}

#[derive(Clone)]
pub struct WithSessionAuthMiddleware<'l> {
    pub login: Cow<'l, str>,
    pub min_role: Role,
}

impl<'l, R> Middleware<R> for WithSessionAuthMiddleware<'l>
where
    R: Registry,
{
    type Error = anyhow::Error;

    fn handle<'a, H, E>(
        &self,
        mut req: R::Request<'a>,
        resp: R::Response<'a>,
        handler: H,
    ) -> Result<Completion, Self::Error>
    where
        R: Registry,
        H: FnOnce(R::Request<'a>, R::Response<'a>) -> Result<Completion, E>,
        E: HandlerError,
    {
        if let Some(role) = get_role(&mut req, None) {
            if role >= self.min_role {
                return handler(req, resp).map_err(|e| anyhow::format_err!("ERROR {}", e));
            }
        }

        let completion = resp
            .redirect(req, self.login.as_ref().to_owned())
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(completion)
    }
}

pub fn get_role<'a>(req: &mut impl Request<'a>, default_role: Option<Role>) -> Option<Role> {
    if let Some(role) = req.attrs().get("role") {
        role.downcast_ref::<Role>().map(Clone::clone)
    } else if let Some(role) = req.session().get("role").ok().flatten() {
        Some(role)
    } else {
        default_role
    }
}

pub fn set_request_role<'a>(req: &mut impl Request<'a>, role: Option<Role>) {
    if let Some(role) = role {
        req.attrs().set("role", Rc::new(role));
    } else {
        req.attrs().remove("role");
    }
}

pub fn set_session_role<'a>(
    req: &mut impl Request<'a>,
    role: Option<Role>,
) -> Result<(), SessionError> {
    if let Some(role) = role {
        req.session().set("role", &role)?;
    } else {
        req.session().remove("role")?;
    }

    Ok(())
}

pub fn register<R, A>(
    registry: &mut R,
    pref: impl AsRef<str>,
    authenticator: A,
) -> Result<(), R::Error>
where
    R: Registry,
    A: Authenticator + 'static,
{
    let prefix = |s| [pref.as_ref(), s].concat();

    registry
        .at(prefix("login"))
        .post(move |req| login(&authenticator, req))?
        .at(prefix("/logout"))
        .post(move |req| logout(req))?;

    Ok(())
}

pub fn login<'a, A>(authenticator: &A, req: &mut impl Request<'a>) -> Result<ResponseData>
where
    A: Authenticator,
{
    if req.session().is_valid() {
        return Ok(().into());
    }

    let bytes: Result<Vec<_>, _> = io::Bytes::<_, 64>::new(req.reader()).take(3000).collect();

    let bytes = bytes.map_err(|e| anyhow::anyhow!(e))?;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct Credentials {
        username: String,
        password: String,
    }

    let credentials: Credentials =
        serde_json::from_slice(&bytes).map_err(|e| anyhow::anyhow!(e))?;

    if let Some(role) = authenticator.authenticate(credentials.username, credentials.password) {
        {
            let mut session = req.session();

            session.invalidate().map_err(|e| anyhow::anyhow!(e))?;
            session
                .create_if_invalid()
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        set_session_role(req, Some(role)).map_err(|e| anyhow::anyhow!(e))?;

        return ResponseData::ok().into();
    }

    ResponseData::new(401)
        .body("Invalid username or password".into())
        .into()
}

pub fn logout<'a>(req: &mut impl Request<'a>) -> Result<ResponseData> {
    req.session().invalidate().map_err(|e| anyhow::anyhow!(e))?;

    ResponseData::ok().into()
}
