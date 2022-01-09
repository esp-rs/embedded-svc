extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;

use anyhow::Result;

use crate::{http::server::registry::*, http::server::*, io, mutex::*, wifi};

use super::{role::Role, *};

pub fn register<R, M, W>(
    registry: &mut R,
    pref: impl AsRef<str>,
    wifi: Arc<M>,
    default_role: Option<Role>,
) -> Result<(), R::Error>
where
    R: Registry,
    M: Mutex<Data = W> + 'static,
    W: wifi::Wifi,
{
    let prefix = |s| [pref.as_ref(), s].concat();

    let wifi_get_status = wifi.clone();
    let wifi_scan = wifi.clone();
    let wifi_get_capabilities = wifi.clone();
    let wifi_get_configuration = wifi.clone();
    let wifi_set_configuration = wifi;

    registry
        .with_middleware(auth::WithRoleMiddleware {
            role: Role::Admin,
            default_role,
        })
        .at(prefix(""))
        .get(move |req| get_status(req, &*wifi_get_status))?
        .at(prefix("/scan"))
        .post(move |req| scan(req, &*wifi_scan))?
        .at(prefix("/caps"))
        .get(move |req| get_capabilities(req, &*wifi_get_capabilities))?
        .at(prefix("/conf"))
        .get(move |req| get_configuration(req, &*wifi_get_configuration))?
        .at(prefix("/conf"))
        .put(move |req| set_configuration(req, &*wifi_set_configuration))?;

    Ok(())
}

fn get_capabilities<'a, M, W>(_req: &mut impl Request<'a>, wifi: &M) -> Result<ResponseData>
where
    M: Mutex<Data = W>,
    W: wifi::Wifi,
{
    let caps = wifi
        .lock()
        .get_capabilities()
        .map_err(|e| anyhow::anyhow!(e))?;

    ResponseData::from_json(&caps)?.into()
}

fn get_status<'a, M, W>(_req: &mut impl Request<'a>, wifi: &M) -> Result<ResponseData>
where
    M: Mutex<Data = W>,
    W: wifi::Wifi,
{
    let status = wifi.lock().get_status();

    ResponseData::from_json(&status)?.into()
}

fn scan<'a, M, W>(_req: &mut impl Request<'a>, wifi: &M) -> Result<ResponseData>
where
    M: Mutex<Data = W>,
    W: wifi::Wifi,
{
    let data = wifi.lock().scan().map_err(|e| anyhow::anyhow!(e))?;

    ResponseData::from_json(&data)?.into()
}

fn get_configuration<'a, M, W>(_req: &mut impl Request<'a>, wifi: &M) -> Result<ResponseData>
where
    M: Mutex<Data = W>,
    W: wifi::Wifi,
{
    let conf = wifi
        .lock()
        .get_configuration()
        .map_err(|e| anyhow::anyhow!(e))?;

    ResponseData::from_json(&conf)?.into()
}

fn set_configuration<'a, M, W>(req: &mut impl Request<'a>, wifi: &M) -> Result<ResponseData>
where
    M: Mutex<Data = W>,
    W: wifi::Wifi,
{
    let bytes: Result<Vec<_>, _> = io::Bytes::<_, 64>::new(req.reader()).take(3000).collect();

    let bytes = bytes?;

    let conf: wifi::Configuration = serde_json::from_slice(&bytes)?;

    wifi.lock()
        .set_configuration(&conf)
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(().into())
}
