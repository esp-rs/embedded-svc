use core::cmp::min;

use crate::errors::wrap::WrapError;
use crate::http::server::registry::Registry;
use crate::http::server::*;
use crate::mutex::*;
use crate::ota::{self, OtaRead, OtaSlot, OtaUpdate};
use crate::utils::json_io;

pub fn register<R, MO, MS, MP, O, S>(
    registry: &mut R,
    ota: MO,
    ota_server: MS,
    progress: MP,
) -> Result<(), R::Error>
where
    R: Registry,
    MO: Mutex<Data = O> + Send + Sync + Clone + 'static,
    MS: Mutex<Data = S> + Send + Sync + Clone + 'static,
    MP: Mutex<Data = Option<usize>> + Send + Sync + Clone + 'static,
    O: ota::Ota,
    S: ota::OtaServer,
{
    let ota_server1 = ota_server.clone();
    let ota_server2 = ota_server.clone();

    let ota1 = ota.clone();
    let ota2 = ota.clone();

    let progress1 = progress.clone();

    registry
        .handle_get("", move |req, resp| get_status(req, resp, &ota1))?
        .handle_get("/updates", move |req, resp| {
            get_updates(req, resp, &ota_server1)
        })?
        .handle_get("/updates/latest", move |req, resp| {
            get_latest_update(req, resp, &ota_server2)
        })?
        .handle_post("/reset", move |req, resp| factory_reset(req, resp, &ota2))?
        .handle_post("/update", move |req, resp| {
            update(req, resp, &ota, &ota_server, &progress1)
        })?
        .handle_get("/update/progress", move |req, resp| {
            get_update_progress(req, resp, &progress)
        })?;

    Ok(())
}

pub fn get_status(
    _req: impl Request,
    resp: impl Response,
    ota: &impl Mutex<Data = impl ota::Ota>,
) -> Result<(), HandlerError> {
    let ota = ota.lock();

    let slot = ota.get_running_slot()?;

    let info = slot.get_firmware_info()?;

    json_io::resp_write::<512, _, _>(resp, &info)?;

    Ok(())
}

pub fn get_updates(
    _req: impl Request,
    resp: impl Response,
    ota_server: &impl Mutex<Data = impl ota::OtaServer>,
) -> Result<(), HandlerError> {
    let mut ota_server = ota_server.lock();

    let updates = ota_server.get_releases()?;

    json_io::resp_write::<512, _, _>(resp, &updates)?;

    Ok(())
}

pub fn get_latest_update(
    _req: impl Request,
    resp: impl Response,
    ota_server: &impl Mutex<Data = impl ota::OtaServer>,
) -> Result<(), HandlerError> {
    let mut ota_server = ota_server.lock();

    let update = ota_server.get_latest_release()?;

    json_io::resp_write::<512, _, _>(resp, &update)?;

    Ok(())
}

pub fn factory_reset(
    _req: impl Request,
    _resp: impl Response,
    ota: &impl Mutex<Data = impl ota::Ota>,
) -> Result<(), HandlerError> {
    ota.lock().factory_reset()?;

    Ok(())
}

pub fn update(
    req: impl Request,
    _resp: impl Response,
    ota: &impl Mutex<Data = impl ota::Ota>,
    ota_server: &impl Mutex<Data = impl ota::OtaServer>,
    progress: &impl Mutex<Data = Option<usize>>,
) -> Result<(), HandlerError> {
    let download_id: Option<heapless::String<128>> = json_io::read::<1024, _, _>(req)?;

    let mut ota_server = ota_server.lock();

    let download_id = match download_id {
        None => ota_server
            .get_latest_release()?
            .and_then(|release| release.download_id),
        some => some,
    };

    let download_id = download_id.ok_or(WrapError("Missing update"))?;

    let mut download_id_arr = [0_u8; 64];

    let did = &mut download_id_arr[..min(64, download_id.len())];
    did.copy_from_slice(&download_id.as_bytes()[..download_id.len()]);

    let mut ota_update = ota_server.open(core::str::from_utf8(did).unwrap())?;

    let size = ota_update.size();

    ota.lock()
        .initiate_update()?
        .update(&mut ota_update, |_, copied| {
            *progress.lock() = size.map(|size| copied as usize * 100 / size as usize)
        })?; // TODO: Take the progress mutex more rarely

    Ok(())
}

pub fn get_update_progress(
    _req: impl Request,
    resp: impl Response,
    progress: &impl Mutex<Data = Option<usize>>,
) -> Result<(), HandlerError> {
    json_io::resp_write::<512, _, _>(resp, &*progress.lock())?;

    Ok(())
}
