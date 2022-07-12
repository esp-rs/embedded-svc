use core::cmp::min;

use crate::errors::wrap::WrapError;
use crate::http::server::*;
use crate::mutex::*;
use crate::ota::{self, OtaRead, OtaSlot, OtaUpdate};
use crate::utils::json_io;

pub fn get_status<C: Connection>(
    connection: &mut C,
    request: C::Request,
    ota: &impl Mutex<Data = impl ota::Ota>,
) -> HandlerResult {
    let ota = ota.lock();

    let slot = ota.get_running_slot()?;

    let info = slot.get_firmware_info()?;

    Ok(json_io::response::<512, _, _>(connection, request, &info)?)
}

pub fn get_updates<C: Connection>(
    connection: &mut C,
    request: C::Request,
    ota_server: &impl Mutex<Data = impl ota::OtaServer>,
) -> HandlerResult {
    let mut ota_server = ota_server.lock();

    let updates = ota_server.get_releases()?;

    Ok(json_io::response::<512, _, _>(
        connection, request, &updates,
    )?)
}

pub fn get_latest_update<C: Connection>(
    connection: &mut C,
    request: C::Request,
    ota_server: &impl Mutex<Data = impl ota::OtaServer>,
) -> HandlerResult {
    let mut ota_server = ota_server.lock();

    let update = ota_server.get_latest_release()?;

    Ok(json_io::response::<512, _, _>(
        connection, request, &update,
    )?)
}

pub fn factory_reset<C: Connection>(
    _connection: &mut C,
    _request: C::Request,
    ota: &impl Mutex<Data = impl ota::Ota>,
) -> HandlerResult {
    ota.lock().factory_reset()?;

    Ok(())
}

pub fn update<C: Connection>(
    connection: &mut C,
    mut request: C::Request,
    ota: &impl Mutex<Data = impl ota::Ota>,
    ota_server: &impl Mutex<Data = impl ota::OtaServer>,
    progress: &impl Mutex<Data = Option<usize>>,
) -> HandlerResult {
    let download_id: Option<heapless::String<128>> =
        json_io::read::<1024, _, _>(connection.reader(&mut request))?;

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

pub fn get_update_progress<C: Connection>(
    connection: &mut C,
    request: C::Request,
    progress: &impl Mutex<Data = Option<usize>>,
) -> HandlerResult {
    Ok(json_io::response::<512, _, _>(
        connection,
        request,
        &*progress.lock(),
    )?)
}
