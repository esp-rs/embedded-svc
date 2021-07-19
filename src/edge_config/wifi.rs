extern crate alloc;

use crate::edge_config::role::Role;

use crate::{httpd::registry::*, httpd::*, wifi::*};

use super::*;

pub fn register<R: Registry>(registry: R, pref: &str, default_role: Option<Role>) -> Result<R> {
    let prefix = |s| [pref.as_ref(), s].concat();

    registry
        .at(prefix(""))
        .get(get_status)?
        .at(prefix("/scan"))
        .post(scan)?
        .at(prefix("/caps"))
        .get(get_capabilities)?
        .at(prefix("/conf"))
        .get(get_configuration)?
        .at(prefix("/conf"))
        .put(set_configuration)?
        .at(pref)
        .middleware(with_permissions(default_role))
}

fn get_capabilities(req: Request) -> Result<Response> {
    let caps = wifi(req, |wifi| wifi.get_capabilities())?;

    json(&caps)
}

fn get_status(req: Request) -> Result<Response> {
    let status = wifi(req, |wifi| wifi.get_status());

    json(&status)
}

fn scan(req: Request) -> Result<Response> {
    let data = wifi_mut(req, |wifi| wifi.scan())?;

    json(&data)
}

fn get_configuration(req: Request) -> Result<Response> {
    let conf = wifi(req, |wifi| wifi.get_configuration())?;

    json(&conf)
}

fn set_configuration(mut req: Request) -> Result<Response> {
    let conf: wifi::Configuration = serde_json::from_slice(req.as_bytes()?.as_slice())?;

    wifi_mut(req, |wifi| wifi.set_configuration(&conf))?;

    Ok(().into())
}

fn with_permissions(
    default_role: Option<Role>,
) -> impl for<'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response> {
    auth::with_role(Role::Admin, default_role)
}

fn wifi<Q>(req: Request, f: impl FnOnce(&dyn Wifi<Error = anyhow::Error>) -> Q) -> Q {
    f(req
        .app()
        .read()
        .unwrap()
        .get("wifi")
        .unwrap()
        .downcast_ref::<Box<dyn Wifi<Error = anyhow::Error>>>()
        .unwrap()
        .as_ref())
}

fn wifi_mut<Q>(req: Request, f: impl FnOnce(&mut dyn Wifi<Error = anyhow::Error>) -> Q) -> Q {
    f(req
        .app()
        .write()
        .unwrap()
        .get_mut("wifi")
        .unwrap()
        .downcast_mut::<Box<dyn Wifi<Error = anyhow::Error>>>()
        .unwrap()
        .as_mut())
}

fn json<T: ?Sized + serde::Serialize>(data: &T) -> Result<Response> {
    Response::ok()
        .content_type("application/json".to_string())
        .body(serde_json::to_string(data)?.into())
        .into()
}
