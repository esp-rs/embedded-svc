use crate::http::server::*;
use crate::mutex::Mutex;
use crate::utils::json_io;
use crate::wifi;

pub fn get_capabilities(
    request: impl Request,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> HandlerResult {
    let caps = wifi.lock().get_capabilities()?;

    Ok(json_io::submit_response::<512, _, _>(
        request.into_response()?,
        &caps,
    )?)
}

pub fn get_status(
    request: impl Request,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> HandlerResult {
    let status = wifi.lock().get_status();

    Ok(json_io::submit_response::<1024, _, _>(
        request.into_response()?,
        &status,
    )?)
}

pub fn scan(request: impl Request, wifi: &impl Mutex<Data = impl wifi::Wifi>) -> HandlerResult {
    let mut wifi = wifi.lock();

    let (aps, _) = wifi.scan_n::<20>()?; // TODO

    Ok(json_io::submit_response::<4096, _, _>(
        request.into_response()?,
        &aps,
    )?)
}

pub fn get_configuration(
    request: impl Request,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> HandlerResult {
    let wifi = wifi.lock();

    let conf = wifi.get_configuration()?;

    Ok(json_io::submit_response::<1024, _, _>(
        request.into_response()?,
        &conf,
    )?)
}

pub fn set_configuration(
    mut request: impl Request,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> HandlerResult {
    let conf: wifi::Configuration = json_io::read::<1024, _, _>(&mut request)?;

    wifi.lock().set_configuration(&conf)?;

    Ok(request.into_response()?.complete()?)
}
