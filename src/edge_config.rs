use crate::httpd::{Request, Response, ResponseBuilder, Result, StateMap};

use http_auth_basic::Credentials;

pub mod wifi;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Role {
    None,
    User,
    Admin
}

pub fn get_role(req: &Request, default_role: Option<Role>) -> Option<Role> {
    if let Some(role) = req.attrs().get("role") {
        return role.downcast_ref::<Role>().map(Clone::clone);
    }

    let session = req.session();

    match session {
        Some(session) => session
            .read()
            .unwrap()
            .get("role")
            .map(|any| any.downcast_ref::<Role>().unwrap().clone())
            .or_else(|| default_role),
        None => None
    }
}

pub fn set_role(state: &mut StateMap, role: Option<Role>) {
    if let Some(role) = role {
        *state.entry("role".to_owned()).or_insert(Box::new(role)) = Box::new(role);
    } else {
        state.remove("role");
    }
}

pub fn with_role(role: Role, default_role: Option<Role>) -> impl for <'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response> {
    move |req, handler| {
        let current_role = get_role(&req, default_role);

        if let Some(current_role) = current_role {
            if current_role >= role {
                return handler(req)
            }
        }

        Ok(400.into())
    }
}

pub fn get_admin_password(req: &Request) -> Option<String> {
    req.app()
        .read()
        .unwrap()
        .get("admin_password")
        .map(|any| any.downcast_ref::<String>().unwrap().clone())
}

pub fn set_admin_password(req: &Request, password: &str) {
    *req.app().write().unwrap().entry("admin_password".to_owned()).or_insert(Box::new(password.to_owned())) = Box::new(password.to_owned());
}

pub fn with_basic_auth(mut req: Request, handler: &dyn Fn(Request) -> Result<Response>) -> Result<Response> {
    if let Some(role) = get_role(&req, None) {
        if role == Role::Admin {
            return handler(req);
        }
    }

    let admin_password = get_admin_password(&req);

    if let Some(admin_password) = admin_password {
        let authorization = req.header("Authorization");
        if let Some(authorization) = authorization {
            if let Ok(credentials) = Credentials::from_header(authorization) {
                if credentials.user_id.to_lowercase() == "admin" && credentials.password == admin_password {
                    set_role(req.attrs_mut(), Some(Role::Admin));

                    return handler(req);
                }
            }
        }
    }

    ResponseBuilder::new(401)
        .header("WWW-Authenticate", "Basic realm=\"User Visible Realm\"")
        .into()
}

pub fn with_session_auth(req: Request, handler: &dyn Fn(Request) -> Result<Response>) -> Result<Response> {
    if let Some(_) = get_role(&req, None) {
        handler(req)
    } else {
        ResponseBuilder::new(301)
            .header("Location", "/login")
            .into()
    }
}
