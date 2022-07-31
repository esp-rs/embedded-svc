use core::cmp::min;

use crate::errors::wrap::WrapError;
use crate::http::server::*;
use crate::mutex::RawMutex;
use crate::ota::{self, OtaRead, OtaUpdate};
use crate::utils::json_io;
use crate::utils::mutex::Mutex;

pub fn get_status(
    request: Request<impl Connection>,
    ota: &Mutex<impl RawMutex, impl ota::Ota>,
) -> HandlerResult {
    let ota = ota.lock();

    let slot = ota.get_running_slot()?;

    Ok(json_io::response::<512, _, _>(request, &slot.firmware)?)
}

pub fn get_updates(
    request: Request<impl Connection>,
    ota_server: &Mutex<impl RawMutex, impl ota::OtaServer>,
) -> HandlerResult {
    let mut ota_server = ota_server.lock();

    let updates = ota_server.get_releases()?;

    Ok(json_io::response::<512, _, _>(request, &updates)?)
}

pub fn get_latest_update(
    request: Request<impl Connection>,
    ota_server: &Mutex<impl RawMutex, impl ota::OtaServer>,
) -> HandlerResult {
    let mut ota_server = ota_server.lock();

    let update = ota_server.get_latest_release()?;

    Ok(json_io::response::<512, _, _>(request, &update)?)
}

pub fn factory_reset(
    _request: Request<impl Connection>,
    ota: &Mutex<impl RawMutex, impl ota::Ota>,
) -> HandlerResult {
    ota.lock().factory_reset()?;

    Ok(())
}

pub fn update(
    mut request: Request<impl Connection>,
    ota: &Mutex<impl RawMutex, impl ota::Ota>,
    ota_server: &Mutex<impl RawMutex, impl ota::OtaServer>,
    progress: &Mutex<impl RawMutex, Option<usize>>,
) -> HandlerResult {
    let download_id: Option<heapless::String<128>> = json_io::read::<1024, _, _>(&mut request)?;

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
    request: Request<impl Connection>,
    progress: &Mutex<impl RawMutex, Option<usize>>,
) -> HandlerResult {
    Ok(json_io::response::<512, _, _>(request, &*progress.lock())?)
}
