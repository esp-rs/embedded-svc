use crate::httpd::{Request, Response};

pub mod wifi;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Role {
    None,
    User,
    Admin
}

pub trait AsRole {
    fn as_role(&self) -> Role;
}

pub fn with_role<R: Request<S, A>, S: AsRole, A>(role: Role, default_role: Role, f: impl Fn(&mut R) -> anyhow::Result<Response<S>>) -> impl Fn(&mut R) -> anyhow::Result<Response<S>> {
    move |req: &mut R| {
        let current_role = req.with_session(|so| so.map_or(default_role, AsRole::as_role));

        if current_role >= role {
            f(req)
        } else {
            Ok(400.into())
        }
    }
}
