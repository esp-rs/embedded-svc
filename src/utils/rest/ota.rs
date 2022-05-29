use core::fmt::Debug;

use crate::errors::{EitherError, EitherError7, Error, ErrorKind};
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

pub fn register<R, MO, MS, MP, O, S>(
    registry: &mut R,
    pref: impl AsRef<str>,
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
    //let prefix = |s| [pref.as_ref(), s].concat();
    let prefix = |s| s;

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
        .at(prefix(""))
        .inline()
        .get(move |req, resp| get_status(req, resp, &ota_get_status))?
        .at(prefix("/updates"))
        .inline()
        .get(move |req, resp| get_updates(req, resp, &otas_get_updates))?
        .at(prefix("/updates/latest"))
        .inline()
        .get(move |req, resp| get_latest_update(req, resp, &otas_get_latest_update))?
        .at(prefix("/reset"))
        .inline()
        .post(move |req, resp| factory_reset(req, resp, &ota_factory_reset))?
        .at(prefix("/update"))
        .inline()
        .post(move |req, resp| update(req, resp, &ota, &otas_update, &progress_update))?
        .at(prefix("/update/progress"))
        .inline()
        .get(move |req, resp| get_update_progress(req, resp, &progress))?;

    Ok(())
}

fn get_status(
    req: impl Request,
    resp: impl Response,
    ota: &impl Mutex<Data = impl ota::Ota>,
) -> Result<Completion, impl Debug> {
    let ota = ota.lock();

    let slot = ota.get_running_slot().map_err(EitherError::Second)?;

    let info = slot.get_firmware_info().map_err(EitherError::Second)?;

    resp.send_json(req, &info).map_err(EitherError::First)
}

fn get_updates(
    req: impl Request,
    resp: impl Response,
    ota_server: &impl Mutex<Data = impl ota::OtaServer>,
) -> Result<Completion, impl Debug> {
    let ota_server = ota_server.lock();

    let updates = ota_server
        .get_releases()
        .map(|releases| releases.collect::<Vec<_>>())
        .map_err(EitherError::Second)?;

    resp.send_json(req, &updates).map_err(EitherError::First)
}

fn get_latest_update(
    req: impl Request,
    resp: impl Response,
    ota_server: &impl Mutex<Data = impl ota::OtaServer>,
) -> Result<Completion, impl Debug> {
    let ota_server = ota_server.lock();

    let update = ota_server
        .get_latest_release()
        .map_err(EitherError::Second)?;

    resp.send_json(req, &update).map_err(EitherError::First)
}

fn factory_reset(
    req: impl Request,
    resp: impl Response,
    ota: &impl Mutex<Data = impl ota::Ota>,
) -> Result<Completion, impl Debug> {
    ota.lock().factory_reset().map_err(EitherError::Second)?;

    resp.submit(req).map_err(EitherError::First)
}

fn update(
    req: impl Request,
    resp: impl Response,
    ota: &impl Mutex<Data = impl ota::Ota>,
    ota_server: &impl Mutex<Data = impl ota::OtaServer>,
    progress: &impl Mutex<Data = Option<usize>>,
) -> Result<Completion, impl Debug> {
    let mut buf = [0_u8; 1000];

    let (buf, _) = read_max(req.reader(), &mut buf).map_err(EitherError7::First)?;

    let download_id: Option<&str> = serde_json::from_slice(buf).map_err(EitherError7::Second)?;

    let ota_server = ota_server.lock();

    let download_id = match download_id {
        None => ota_server
            .get_latest_release()
            .map_err(EitherError7::Third)?
            .and_then(|release| release.download_id),
        some => some,
    };

    let download_id = download_id.ok_or_else(|| EitherError7::Fourth(MissingUpdateError))?;

    let mut ota_update = ota_server.open(download_id).map_err(EitherError7::Third)?;

    let size = ota_update.size();

    ota.lock()
        .initiate_update()
        .map_err(EitherError7::Seventh)?
        .update(&mut ota_update, |_, copied| {
            *progress.lock() = size.map(|size| copied as usize * 100 / size as usize)
        }) // TODO: Take the progress mutex more rarely
        .map_err(EitherError7::Fifth)?;

    resp.submit(req).map_err(EitherError7::Sixth)
}

fn get_update_progress(
    req: impl Request,
    resp: impl Response,
    progress: &impl Mutex<Data = Option<usize>>,
) -> Result<Completion, impl Debug> {
    resp.send_json(req, &*progress.lock())
}
