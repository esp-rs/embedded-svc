use crate::http::server::registry::Registry;
use crate::http::server::*;
use crate::mutex::Mutex;
use crate::utils::json_io;
use crate::wifi;

pub fn register<R, M, W>(registry: &mut R, wifi: M) -> Result<(), R::Error>
where
    R: Registry,
    M: Mutex<Data = W> + Clone + Send + Sync + 'static,
    W: wifi::Wifi,
{
    let wifi1 = wifi.clone();
    let wifi2 = wifi.clone();
    let wifi3 = wifi.clone();
    let wifi4 = wifi.clone();

    registry
        .handle_get("", move |req, resp| get_status(req, resp, &wifi1))?
        .handle_post("/scan", move |req, resp| scan(req, resp, &wifi2))?
        .handle_get("/caps", move |req, resp| {
            get_capabilities(req, resp, &wifi3)
        })?
        .handle_get("/conf", move |req, resp| {
            get_configuration(req, resp, &wifi4)
        })?
        .handle_put("/conf", move |req, resp| {
            set_configuration(req, resp, &wifi)
        })?;

    Ok(())
}

pub fn get_capabilities(
    _req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<(), HandlerError> {
    let caps = wifi.lock().get_capabilities()?;

    json_io::resp_write::<512, _, _>(resp, &caps)?;

    Ok(())
}

pub fn get_status(
    _req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<(), HandlerError> {
    let status = wifi.lock().get_status();

    json_io::resp_write::<1024, _, _>(resp, &status)?;

    Ok(())
}

pub fn scan(
    _req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<(), HandlerError> {
    let mut wifi = wifi.lock();

    let (aps, _) = wifi.scan_n::<20>()?; // TODO

    json_io::resp_write::<4096, _, _>(resp, &aps)?;

    Ok(())
}

pub fn get_configuration(
    _req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<(), HandlerError> {
    let wifi = wifi.lock();

    let conf = wifi.get_configuration()?;

    json_io::resp_write::<1024, _, _>(resp, &conf)?;

    Ok(())
}

pub fn set_configuration(
    req: impl Request,
    _resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<(), HandlerError> {
    let conf: wifi::Configuration = json_io::read::<1024, _, _>(req)?;

    wifi.lock().set_configuration(&conf)?;

    Ok(())
}
