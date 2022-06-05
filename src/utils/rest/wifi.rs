use core::fmt::Debug;
use core::mem::MaybeUninit;

use crate::errors::{EitherError, EitherError4};
use crate::io::read_max;
use crate::mutex::Mutex;
use crate::wifi::{self, AccessPointInfo};
use crate::{http::server::registry::*, http::server::*};

use crate::utils::role::*;

pub fn register<R, W, T>(
    registry: &mut R,
    wifi: W,
    default_role: Option<Role>,
) -> Result<(), R::Error>
where
    R: Registry,
    W: Mutex<Data = T> + Send + Sync + Clone + 'static,
    T: wifi::Wifi,
{
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
        .at("/scan")
        .inline()
        .post(move |req, resp| scan(req, resp, &wifi_scan))?
        .at("/caps")
        .inline()
        .get(move |req, resp| get_capabilities(req, resp, &wifi_get_capabilities))?
        .at("/conf")
        .inline()
        .get(move |req, resp| get_configuration(req, resp, &wifi_get_configuration))?
        .at("/conf")
        .inline()
        .put(move |req, resp| set_configuration(req, resp, &wifi_set_configuration))?
        .at("")
        .inline()
        .get(move |req, resp| get_status(req, resp, &wifi_get_status))?;

    Ok(())
}

fn get_capabilities(
    req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, impl Debug> {
    let caps = wifi.lock().get_capabilities().map_err(EitherError::E1)?;

    resp.send_json(req, &caps).map_err(EitherError::E2)
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
    let mut aps: [MaybeUninit<AccessPointInfo<heapless::String<64>>>; 20] =
        unsafe { MaybeUninit::uninit().assume_init() };

    let mut wifi = wifi.lock();

    let (aps, _) = wifi.scan_fill(&mut aps).map_err(EitherError::E1)?;

    resp.send_json(req, aps).map_err(EitherError::E2)
}

fn get_configuration(
    req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, impl Debug> {
    let wifi = wifi.lock();

    let conf = wifi
        .get_configuration::<heapless::String<64>>()
        .map_err(EitherError::E1)?;

    resp.send_json(req, &conf).map_err(EitherError::E2)
}

fn set_configuration(
    req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, impl Debug> {
    let mut buf = [0_u8; 1000];

    let (buf, _) = read_max(req.reader(), &mut buf).map_err(EitherError4::E1)?;

    let conf: wifi::Configuration<&str> = serde_json::from_slice(buf).map_err(EitherError4::E2)?;

    wifi.lock()
        .set_configuration(&conf)
        .map_err(EitherError4::E3)?;

    resp.submit(req).map_err(EitherError4::E4)
}
