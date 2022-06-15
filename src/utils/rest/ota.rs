use core::cmp::min;
use core::fmt::Debug;

use crate::errors::wrap::{EitherError, EitherError3, EitherError8, WrapError};
use crate::http::server::*;
use crate::io::read_max;
use crate::mutex::*;
use crate::ota::{self, OtaRead, OtaSlot, OtaUpdate};

pub fn get_status(
    req: impl Request,
    resp: impl Response,
    ota: &impl Mutex<Data = impl ota::Ota>,
) -> Result<Completion, impl Debug> {
    let ota = ota.lock();

    let slot = ota.get_running_slot().map_err(EitherError3::E1)?;

    let info = slot.get_firmware_info().map_err(EitherError3::E2)?;

    resp.send_json(req, &info).map_err(EitherError3::E3)
}

pub fn get_updates(
    req: impl Request,
    resp: impl Response,
    ota_server: &impl Mutex<Data = impl ota::OtaServer>,
) -> Result<Completion, impl Debug> {
    let mut ota_server = ota_server.lock();

    let updates = ota_server.get_releases().map_err(EitherError::E2)?;

    resp.send_json(req, &updates).map_err(EitherError::E1)
}

pub fn get_latest_update(
    req: impl Request,
    resp: impl Response,
    ota_server: &impl Mutex<Data = impl ota::OtaServer>,
) -> Result<Completion, impl Debug> {
    let mut ota_server = ota_server.lock();

    let update = ota_server.get_latest_release().map_err(EitherError::E2)?;

    resp.send_json(req, &update).map_err(EitherError::E1)
}

pub fn factory_reset(
    req: impl Request,
    resp: impl Response,
    ota: &impl Mutex<Data = impl ota::Ota>,
) -> Result<Completion, impl Debug> {
    ota.lock().factory_reset().map_err(EitherError::E2)?;

    resp.submit(req).map_err(EitherError::E1)
}

pub fn update(
    mut req: impl Request,
    resp: impl Response,
    ota: &impl Mutex<Data = impl ota::Ota>,
    ota_server: &impl Mutex<Data = impl ota::OtaServer>,
    progress: &impl Mutex<Data = Option<usize>>,
) -> Result<Completion, impl Debug> {
    let mut buf = [0_u8; 1000]; // TODO

    let (buf, _) = read_max(req.reader(), &mut buf).map_err(EitherError8::E1)?;

    let download_id: Option<heapless::String<128>> =
        serde_json::from_slice(buf).map_err(EitherError8::E2)?;

    let mut ota_server = ota_server.lock();

    let download_id = match download_id {
        None => ota_server
            .get_latest_release()
            .map_err(EitherError8::E3)?
            .and_then(|release| release.download_id),
        some => some,
    };

    let download_id = download_id.ok_or_else(|| EitherError8::E4(WrapError("Missing update")))?;

    let mut download_id_arr = [0_u8; 64];

    let did = &mut download_id_arr[..min(64, download_id.len())];
    did.copy_from_slice(&download_id.as_bytes()[..download_id.len()]);

    let mut ota_update = ota_server
        .open(core::str::from_utf8(did).unwrap())
        .map_err(EitherError8::E8)?;

    let size = ota_update.size();

    ota.lock()
        .initiate_update()
        .map_err(EitherError8::E7)?
        .update(&mut ota_update, |_, copied| {
            *progress.lock() = size.map(|size| copied as usize * 100 / size as usize)
        }) // TODO: Take the progress mutex more rarely
        .map_err(EitherError8::E5)?;

    resp.submit(req).map_err(EitherError8::E6)
}

pub fn get_update_progress(
    req: impl Request,
    resp: impl Response,
    progress: &impl Mutex<Data = Option<usize>>,
) -> Result<Completion, impl Debug> {
    resp.send_json(req, &*progress.lock())
}
