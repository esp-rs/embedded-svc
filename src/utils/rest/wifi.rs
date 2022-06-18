use crate::http::server::registry::Registry;
use crate::http::server::*;
use crate::io::read_max;
use crate::mutex::Mutex;
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
) -> Result<Completion, HandlerError> {
    let caps = wifi.lock().get_capabilities()?;

    Ok(resp.send_json(&caps)?)
}

pub fn get_status(
    _req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, HandlerError> {
    let status = wifi.lock().get_status();

    Ok(resp.send_json(&status)?)
}

pub fn scan(
    _req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, HandlerError> {
    let mut wifi = wifi.lock();

    let (aps, _) = wifi.scan_n::<20>()?; // TODO

    Ok(resp.send_json(&aps)?)
}

pub fn get_configuration(
    _req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, HandlerError> {
    let wifi = wifi.lock();

    let conf = wifi.get_configuration()?;

    Ok(resp.send_json(&conf)?)
}

pub fn set_configuration(
    mut req: impl Request,
    resp: impl Response,
    wifi: &impl Mutex<Data = impl wifi::Wifi>,
) -> Result<Completion, HandlerError> {
    let mut buf = [0_u8; 1000]; // TODO

    let (buf, _) = read_max(req.reader(), &mut buf)?;

    let conf: wifi::Configuration = serde_json::from_slice(buf)?;

    wifi.lock().set_configuration(&conf)?;

    Ok(resp.submit()?)
}
