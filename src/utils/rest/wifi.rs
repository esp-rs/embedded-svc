use core::fmt::Debug;

use crate::errors::wrap::{EitherError, EitherError4};
use crate::http::server::*;
use crate::io::read_max;
use crate::mutex::Mutex;
use crate::wifi;

pub fn get_capabilities(
    req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, impl Debug> {
    let caps = wifi.lock().get_capabilities().map_err(EitherError::E1)?;

    resp.send_json(req, &caps).map_err(EitherError::E2)
}

pub fn get_status(
    req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, impl Debug> {
    let status = wifi.lock().get_status();

    resp.send_json(req, &status)
}

pub fn scan(
    req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, impl Debug> {
    let mut wifi = wifi.lock();

    let (aps, _) = wifi.scan_n::<20>().map_err(EitherError::E1)?; // TODO

    resp.send_json(req, &aps).map_err(EitherError::E2)
}

pub fn get_configuration(
    req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, impl Debug> {
    let wifi = wifi.lock();

    let conf = wifi.get_configuration().map_err(EitherError::E1)?;

    resp.send_json(req, &conf).map_err(EitherError::E2)
}

pub fn set_configuration(
    mut req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, impl Debug> {
    let mut buf = [0_u8; 1000]; // TODO

    let (buf, _) = read_max(req.reader(), &mut buf).map_err(EitherError4::E1)?;

    let conf: wifi::Configuration = serde_json::from_slice(buf).map_err(EitherError4::E2)?;

    wifi.lock()
        .set_configuration(&conf)
        .map_err(EitherError4::E3)?;

    resp.submit(req).map_err(EitherError4::E4)
}
