extern crate alloc;
use alloc::sync::Arc;

use crate::edge_config::role::Role;

use crate::{httpd::registry::*, httpd::*, mutex::*, wifi};

use super::*;

pub fn register<R, M, W, E>(
    registry: R,
    pref: &str,
    wifi: Arc<M>,
    default_role: Option<Role>,
) -> Result<R>
where
    R: Registry,
    M: Mutex<Data = W> + 'static,
    W: wifi::Wifi<Error = E>,
    E: Into<anyhow::Error>,
{
    let prefix = |s| [pref, s].concat();

    let wifi_get_status = wifi.clone();
    let wifi_scan = wifi.clone();
    let wifi_get_capabilities = wifi.clone();
    let wifi_get_configuration = wifi.clone();
    let wifi_set_configuration = wifi;

    registry
        .at(prefix(""))
        .get(move |req| get_status(req, &*wifi_get_status))?
        .at(prefix("/scan"))
        .post(move |req| scan(req, &*wifi_scan))?
        .at(prefix("/caps"))
        .get(move |req| get_capabilities(req, &*wifi_get_capabilities))?
        .at(prefix("/conf"))
        .get(move |req| get_configuration(req, &*wifi_get_configuration))?
        .at(prefix("/conf"))
        .put(move |req| set_configuration(req, &*wifi_set_configuration))?
        .at(pref)
        .middleware(with_permissions(default_role))
}

fn get_capabilities<M, W, E>(_req: Request, wifi: &M) -> Result<Response>
where
    M: Mutex<Data = W>,
    W: wifi::Wifi<Error = E>,
    E: Into<anyhow::Error>,
{
    let caps = wifi
        .with_lock(|wifi| wifi.get_capabilities())
        .map_err(Into::into)?;

    json(&caps)
}

fn get_status<M, W, E>(_req: Request, wifi: &M) -> Result<Response>
where
    M: Mutex<Data = W>,
    W: wifi::Wifi<Error = E>,
    E: Into<anyhow::Error>,
{
    let status = wifi.with_lock(|wifi| wifi.get_status());

    json(&status)
}

fn scan<M, W, E>(_req: Request, wifi: &M) -> Result<Response>
where
    M: Mutex<Data = W>,
    W: wifi::Wifi<Error = E>,
    E: Into<anyhow::Error>,
{
    let data = wifi.with_lock(|wifi| wifi.scan()).map_err(Into::into)?;

    json(&data)
}

fn get_configuration<M, W, E>(_req: Request, wifi: &M) -> Result<Response>
where
    M: Mutex<Data = W>,
    W: wifi::Wifi<Error = E>,
    E: Into<anyhow::Error>,
{
    let conf = wifi
        .with_lock(|wifi| wifi.get_configuration())
        .map_err(Into::into)?;

    json(&conf)
}

fn set_configuration<M, W, E>(mut req: Request, wifi: &M) -> Result<Response>
where
    M: Mutex<Data = W>,
    W: wifi::Wifi<Error = E>,
    E: Into<anyhow::Error>,
{
    let conf: wifi::Configuration = serde_json::from_slice(req.as_bytes()?.as_slice())?;

    wifi.with_lock(|wifi| wifi.set_configuration(&conf))
        .map_err(Into::into)?;

    Ok(().into())
}

fn with_permissions(
    default_role: Option<Role>,
) -> impl for<'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response> {
    auth::with_role(Role::Admin, default_role)
}

fn json<T: ?Sized + serde::Serialize>(data: &T) -> Result<Response> {
    Response::ok()
        .content_type("application/json".to_string())
        .body(serde_json::to_string(data)?.into())
        .into()
}
