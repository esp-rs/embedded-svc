use core::fmt::Debug;

use crate::errors::{EitherError, EitherError4};
use crate::io::read_max;
use crate::mutex::Mutex;
use crate::wifi::{self, AccessPointInfo};
use crate::{http::server::registry::*, http::server::*};

use crate::utils::role::*;

pub fn register<R, W, T>(
    registry: &mut R,
    pref: impl AsRef<str>,
    wifi: W,
    default_role: Option<Role>,
) -> Result<(), R::Error>
where
    R: Registry,
    W: Mutex<Data = T> + Send + Sync + Clone + 'static,
    T: wifi::Wifi,
{
    //let prefix = |s| [pref.as_ref(), s].concat();
    let prefix = |s| s;

    let wifi_get_status = wifi.clone();
    let wifi_scan = wifi.clone();
    let wifi_get_capabilities = wifi.clone();
    let wifi_get_configuration = wifi.clone();
    let wifi_set_configuration = wifi;

    registry
        .with_middleware(super::auth::WithRoleMiddleware {
            role: Role::Admin,
            default_role,
        })
        .at(prefix(""))
        .inline()
        .get(move |req, resp| get_status(req, resp, &wifi_get_status))?
        .at(prefix("/scan"))
        .inline()
        .post(move |req, resp| scan(req, resp, &wifi_scan))?
        .at(prefix("/caps"))
        .inline()
        .get(move |req, resp| get_capabilities(req, resp, &wifi_get_capabilities))?
        .at(prefix("/conf"))
        .inline()
        .get(move |req, resp| get_configuration(req, resp, &wifi_get_configuration))?
        .at(prefix("/conf"))
        .inline()
        .put(move |req, resp| set_configuration(req, resp, &wifi_set_configuration))?;

    Ok(())
}

fn get_capabilities(
    req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, impl Debug> {
    let caps = wifi.lock().get_capabilities().map_err(EitherError::First)?;

    resp.send_json(req, &caps).map_err(EitherError::Second)
}

fn get_status(
    req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, impl Debug> {
    let status = wifi.lock().get_status();

    resp.send_json(req, &status)
}

fn scan(
    req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, impl Debug> {
    let mut aps: [AccessPointInfo; 20] = [Default::default(); 20];

    let mut wifi = wifi.lock();

    let (aps, _) = wifi.scan_fill(&mut aps).map_err(EitherError::First)?;

    resp.send_json(req, aps).map_err(EitherError::Second)
}

fn get_configuration(
    req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, impl Debug> {
    let wifi = wifi.lock();

    let conf = wifi.get_configuration().map_err(EitherError::First)?;

    resp.send_json(req, &conf).map_err(EitherError::Second)
}

fn set_configuration(
    req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, impl Debug> {
    let mut buf = [0_u8; 1000];

    let size = read_max(req.reader(), &mut buf).map_err(EitherError4::First)?;

    let conf: wifi::Configuration<&str> =
        serde_json::from_slice(&buf[..size]).map_err(EitherError4::Second)?;

    wifi.lock()
        .set_configuration(&conf)
        .map_err(EitherError4::Third)?;

    resp.submit(req).map_err(EitherError4::Fourth)
}
