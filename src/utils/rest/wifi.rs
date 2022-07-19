use crate::http::server::*;
use crate::mutex::Mutex;
use crate::utils::json_io;
use crate::wifi;

pub fn get_capabilities<C: Connection>(
    connection: &mut C,
    request: C::Request,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> HandlerResult {
    let caps = wifi.lock().get_capabilities()?;

    Ok(json_io::response::<512, _, _>(connection, request, &caps)?)
}

pub fn get_status<C: Connection>(
    connection: &mut C,
    request: C::Request,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> HandlerResult {
    let status = wifi.lock().get_status();

    Ok(json_io::response::<1024, _, _>(
        connection, request, &status,
    )?)
}

pub fn scan<C: Connection>(
    connection: &mut C,
    request: C::Request,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> HandlerResult {
    let mut wifi = wifi.lock();

    let (aps, _) = wifi.scan_n::<20>()?; // TODO

    Ok(json_io::response::<4096, _, _>(connection, request, &aps)?)
}

pub fn get_configuration<C: Connection>(
    connection: &mut C,
    request: C::Request,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> HandlerResult {
    let wifi = wifi.lock();

    let conf = wifi.get_configuration()?;

    Ok(json_io::response::<1024, _, _>(connection, request, &conf)?)
}

pub fn set_configuration<C: Connection>(
    connection: &mut C,
    mut request: C::Request,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> HandlerResult {
    let conf: wifi::Configuration = json_io::read::<1024, _, _>(connection.reader(&mut request))?;

    wifi.lock().set_configuration(&conf)?;

    Ok(())
}
