use core::cmp::min;
use core::fmt::Debug;

use crate::errors::{
    either::{EitherError, EitherError8},
    Error, ErrorKind,
};
use crate::http::server::registry::*;
use crate::http::server::*;
use crate::io::read_max;
use crate::mutex::*;
use crate::ota::{self, OtaRead, OtaSlot, OtaUpdate};

use crate::utils::role::*;

#[derive(Debug)]
pub struct MissingUpdateError;

impl core::fmt::Display for MissingUpdateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "MissingUpdateError")
    }
}

impl Error for MissingUpdateError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MissingUpdateError {
    // TODO
    // fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    //     match self {
    //         CopyError::ReadError(r) => Some(r),
    //         CopyError::WriteError(w) => Some(w),
    //     }
    // }
}

pub fn register<'a, R, MO, MS, MP, O, S>(
    registry: &mut R,
    ota: MO,
    ota_server: MS,
    progress: MP,
    default_role: Option<Role>,
) -> Result<(), R::Error>
where
    R: Registry,
    MO: Mutex<Data = O> + Send + Sync + Clone + 'static,
    MS: Mutex<Data = S> + Send + Sync + Clone + 'static,
    MP: Mutex<Data = Option<usize>> + Send + Sync + Clone + 'static,
    O: ota::Ota,
    S: ota::OtaServer,
{
    let otas_get_updates = ota_server.clone();
    let otas_get_latest_update = ota_server.clone();
    let otas_update = ota_server;
    let ota_get_status = ota.clone();
    let ota_factory_reset = ota.clone();
    let progress_update = progress.clone();

    registry
        .with_middleware(super::auth::WithRoleMiddleware {
            role: Role::Admin,
            default_role,
        })
        .at("/reset")
        .inline()
        .post(move |req, resp| factory_reset(req, resp, &ota_factory_reset))?
        .at("/updates/latest")
        .inline()
        .get(move |req, resp| get_latest_update(req, resp, &otas_get_latest_update))?
        .at("/update/progress")
        .inline()
        .get(move |req, resp| get_update_progress(req, resp, &progress))?
        .at("/updates")
        .inline()
        .get(move |req, resp| get_updates(req, resp, &otas_get_updates))?
        .at("/update")
        .inline()
        .post(move |req, resp| update(req, resp, &ota, &otas_update, &progress_update))?
        .at("")
        .inline()
        .get(move |req, resp| get_status(req, resp, &ota_get_status))?;

    Ok(())
}

fn get_status(
    req: impl Request,
    resp: impl Response,
    ota: &impl Mutex<Data = impl ota::Ota>,
) -> Result<Completion, impl Debug> {
    let ota = ota.lock();

    let slot = ota.get_running_slot().map_err(EitherError::E2)?;

    let info = slot.get_firmware_info::<&str>().map_err(EitherError::E2)?;

    resp.send_json(req, &info).map_err(EitherError::E1)
}

fn get_updates<'a>(
    req: impl Request,
    resp: impl Response,
    ota_server: &impl Mutex<Data = impl ota::OtaServer>,
) -> Result<Completion, impl Debug> {
    let mut ota_server = ota_server.lock();

    let updates = ota_server.get_releases().map_err(EitherError::E2)?;

    resp.send_json(req, &updates).map_err(EitherError::E1)
}

fn get_latest_update(
    req: impl Request,
    resp: impl Response,
    ota_server: &impl Mutex<Data = impl ota::OtaServer>,
) -> Result<Completion, impl Debug> {
    let mut ota_server = ota_server.lock();

    let update = ota_server
        .get_latest_release::<&str>()
        .map_err(EitherError::E2)?;

    resp.send_json(req, &update).map_err(EitherError::E1)
}

fn factory_reset(
    req: impl Request,
    resp: impl Response,
    ota: &impl Mutex<Data = impl ota::Ota>,
) -> Result<Completion, impl Debug> {
    ota.lock().factory_reset().map_err(EitherError::E2)?;

    resp.submit(req).map_err(EitherError::E1)
}

fn update<'a>(
    req: impl Request,
    resp: impl Response,
    ota: &impl Mutex<Data = impl ota::Ota>,
    ota_server: &impl Mutex<Data = impl ota::OtaServer>,
    progress: &impl Mutex<Data = Option<usize>>,
) -> Result<Completion, impl Debug> {
    let mut buf = [0_u8; 1000];

    let (buf, _) = read_max(req.reader(), &mut buf).map_err(EitherError8::E1)?;

    let download_id: Option<&str> = serde_json::from_slice(buf).map_err(EitherError8::E2)?;

    let mut ota_server = ota_server.lock();

    let download_id = match download_id {
        None => ota_server
            .get_latest_release()
            .map_err(EitherError8::E3)?
            .and_then(|release| release.download_id),
        some => some,
    };

    let download_id = download_id.ok_or_else(|| EitherError8::E4(MissingUpdateError))?;

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

fn get_update_progress(
    req: impl Request,
    resp: impl Response,
    progress: &impl Mutex<Data = Option<usize>>,
) -> Result<Completion, impl Debug> {
    resp.send_json(req, &*progress.lock())
}
