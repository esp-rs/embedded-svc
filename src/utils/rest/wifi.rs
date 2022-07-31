use crate::http::server::*;
use crate::mutex::RawMutex;
use crate::utils::json_io;
use crate::utils::mutex::Mutex;
use crate::wifi;

pub fn get_capabilities(
    request: Request<impl Connection>,
    wifi: &Mutex<impl RawMutex, impl wifi::Wifi>,
) -> HandlerResult {
    let caps = wifi.lock().get_capabilities()?;

    Ok(json_io::response::<512, _, _>(request, &caps)?)
}

pub fn get_status(
    request: Request<impl Connection>,
    wifi: &Mutex<impl RawMutex, impl wifi::Wifi>,
) -> HandlerResult {
    let status = wifi.lock().get_status();

    Ok(json_io::response::<1024, _, _>(request, &status)?)
}

pub fn scan(
    request: Request<impl Connection>,
    wifi: &Mutex<impl RawMutex, impl wifi::Wifi>,
) -> HandlerResult {
    let mut wifi = wifi.lock();

    let (aps, _) = wifi.scan_n::<20>()?; // TODO

    Ok(json_io::response::<4096, _, _>(request, &aps)?)
}

pub fn get_configuration(
    request: Request<impl Connection>,
    wifi: &Mutex<impl RawMutex, impl wifi::Wifi>,
) -> HandlerResult {
    let wifi = wifi.lock();

    let conf = wifi.get_configuration()?;

    Ok(json_io::response::<1024, _, _>(request, &conf)?)
}

pub fn set_configuration(
    mut request: Request<impl Connection>,
    wifi: &Mutex<impl RawMutex, impl wifi::Wifi>,
) -> HandlerResult {
    let conf: wifi::Configuration = json_io::read::<1024, _, _>(&mut request)?;

    wifi.lock().set_configuration(&conf)?;

    Ok(())
}
