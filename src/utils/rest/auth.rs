use http_auth_basic::Credentials;

use super::role::*;

use crate::httpd::{Request, Response, Result, SessionState, StateMap};

pub fn get_role(req: &Request, default_role: Option<Role>) -> Option<Role> {
    if let Some(role) = req.attrs().get("role") {
        return role.downcast_ref::<Role>().map(Clone::clone);
    }

    match req.session() {
        Some(session) => session
            .read()
            .unwrap()
            .get("role")
            .map(|any| *any.downcast_ref::<Role>().unwrap())
            .or(default_role),
        None => None,
    }
}

pub fn set_role(state: &mut StateMap, role: Option<Role>) {
    if let Some(role) = role {
        state
            .entry("role".to_owned())
            .or_insert_with(|| Box::new(role));
    } else {
        state.remove("role");
    }
}

pub fn with_role(
    role: Role,
    default_role: Option<Role>,
) -> impl for<'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response> {
    move |req, handler| {
        let current_role = get_role(&req, default_role);

        if let Some(current_role) = current_role {
            if current_role >= role {
                return handler(req);
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
    req.app()
        .write()
        .unwrap()
        .entry("admin_password".to_owned())
        .or_insert_with(|| Box::new(password.to_owned()));
}

pub fn with_basic_auth(
    mut req: Request,
    handler: &dyn Fn(Request) -> Result<Response>,
) -> Result<Response> {
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
                if credentials.user_id.to_lowercase() == ADMIN_USERNAME
                    && credentials.password == admin_password
                {
                    set_role(req.attrs_mut(), Some(Role::Admin));

                    return handler(req);
                }
            }
        }
    }

    Response::new(401)
        .header("WWW-Authenticate", "Basic realm=\"User Visible Realm\"")
        .into()
}

pub fn login(mut req: Request) -> Result<Response> {
    if req.session().is_some() {
        return Ok(().into());
    }

    let admin_password = get_admin_password(&req);
    if admin_password == None {
        return Ok(().into());
    }

    let bytes = req.as_bytes()?;

    let mut username = None;
    let mut password = None;

    for (key, value) in url::form_urlencoded::parse(&bytes).into_owned() {
        if key == "username" {
            username = Some(value);
        } else if key == "password" {
            password = Some(value);
        }
    }

    if username.map(|s| s.to_lowercase()) == Some(ADMIN_USERNAME.to_owned())
        && password == admin_password
    {
        let mut session_state = StateMap::new();
        set_role(&mut session_state, Some(Role::Admin));

        Response::ok()
            .new_session_state(SessionState::New(session_state))
            .into()
    } else {
        Response::new(401)
            .body("Invalid username or password".into())
            .into()
    }
}

pub fn logout(_req: Request) -> Result<Response> {
    Response::ok()
        .new_session_state(SessionState::Invalidate)
        .into()
}

pub fn with_session_auth(
    login: impl Into<String>,
) -> impl for<'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response> {
    let login = login.into();

    move |req, handler| {
        if get_role(&req, None).is_some() {
            handler(req)
        } else {
            Response::redirect(login.clone()).into()
        }
    }
}
