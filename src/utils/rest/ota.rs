extern crate alloc;
use alloc::vec::Vec;
use alloc::{string::String, sync::Arc};

use crate::errors::{EitherError, EitherError3, EitherError4};
use crate::http::server::registry::*;
use crate::http::server::*;
use crate::io;
use crate::mutex::*;
use crate::ota::{self, OtaRead, OtaSlot, OtaUpdate};

use crate::utils::role::*;

pub fn register<R, MO, MS, MP, O, S>(
    registry: &mut R,
    pref: impl AsRef<str>,
    ota: Arc<MO>,
    ota_server: Arc<MS>,
    progress: Arc<MP>,
    default_role: Option<Role>,
) -> Result<(), R::Error>
where
    R: Registry,
    MO: Mutex<Data = O> + 'static,
    MS: Mutex<Data = S> + 'static,
    MP: Mutex<Data = Option<f32>> + 'static,
    O: ota::Ota,
    S: ota::OtaServer,
{
    let prefix = |s| [pref.as_ref(), s].concat();

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
        .get(move |req| get_status(req, &*ota_get_status))?
        .at(prefix("/updates"))
        .get(move |req| get_updates(req, &*otas_get_updates))?
        .at(prefix("/updates/latest"))
        .get(move |req| get_latest_update(req, &*otas_get_latest_update))?
        .at(prefix("/reset"))
        .post(move |req| factory_reset(req, &*ota_factory_reset))?
        .at(prefix("/update"))
        .post(move |req| update(req, &*ota, &*otas_update, &*progress_update))?
        .at(prefix("/update/progress"))
        .get(move |req| get_update_progress(req, &*progress))?;

    Ok(())
}

fn get_status<M, O, R>(_req: &mut R, ota: &M) -> Result<ResponseData, EitherError<serde_json::Error, O::Error>>
where
    M: Mutex<Data = O>,
    O: ota::Ota,
    R: Request,
{
    let info = ota
        .lock()
        .get_running_slot()
        .map_err(|e| EitherError::Second)
        .and_then(|slot| slot.get_firmware_info().map_err(EitherError::Second))?;

    ResponseData::from_json(&info)
        .map_err(EitherError::First)?
        .into()
}

fn get_updates<'a, M, O, R>(_req: &mut R, ota_server: &M) -> Result<ResponseData, EitherError<serde_json::Error, O::Error>>
where
    M: Mutex<Data = O>,
    O: ota::OtaServer,
    R: Request<'a>,
{
    let updates = ota_server
        .lock()
        .get_releases()
        .map(|releases| releases.collect::<Vec<_>>())
        .map_err(|e| anyhow::anyhow!(e))?;

    ResponseData::from_json(&updates)
        .map_err(EitherError::First)?
        .into()
}

fn get_latest_update<'a, M, O, R>(_req: &mut R, ota_server: &M) -> Result<ResponseData, EitherError<serde_json::Error, O::Error>>
where
    M: Mutex<Data = O>,
    O: ota::OtaServer,
    R: Request<'a>,
{
    let update = ota_server
        .lock()
        .get_latest_release()
        .map_err(EitherError::Second)?;

    ResponseData::from_json(&update)
        .map_err(EitherError::First)?
        .into()
}

fn factory_reset<'a, M, O, R>(_req: &mut R, ota: &M) -> Result<ResponseData, O::Error>
where
    M: Mutex<Data = O>,
    O: ota::Ota,
    R: Request<'a>,
{
    ota.lock().factory_reset().map_err(EitherError::First)?;

    Ok(ResponseData::ok())
}

fn update<'a, MO, MS, MP, O, S, R>(
    req: &mut R,
    ota: &MO,
    ota_server: &MS,
    progress: &MP,
) -> Result<ResponseData, EitherError4<serde_json::Error, O::Error, S::Error, R::Error>>
where
    MO: Mutex<Data = O>,
    MS: Mutex<Data = S>,
    MP: Mutex<Data = Option<f32>>,
    O: ota::Ota,
    S: ota::OtaServer,
    R: Request<'a>,
{
    let bytes: Result<Vec<_>, _> = io::Bytes::<_, 64>::new(req.reader()).take(3000).collect();

    let bytes = bytes.map_err(EitherError4::Fourth)?;

    let download_id: Option<&'a str> =
        serde_json::from_slice(&bytes).map_err(EitherError4::First)?;

    let mut ota_server = ota_server.lock();

    let download_id = match download_id {
        None => ota_server
            .get_latest_release()
            .map_err(EitherError4::Third)?
            .and_then(|release| release.download_id),
        some => some,
    };

    let download_id = download_id.ok_or_else(|| anyhow!("No update"))?;

    let mut ota_update = ota_server
        .open(download_id)
        .map_err(|e| anyhow::anyhow!(e))?;
    let size = ota_update.size();

    ota.lock()
        .initiate_update()
        .map_err(|e| anyhow::anyhow!(e))?
        .update(&mut ota_update, |_, copied| {
            *progress.lock() = size.map(|size| copied as f32 / size as f32)
        }) // TODO: Take the progress mutex more rarely
        .map_err(EitherError4::Second)?;

    Ok(().into())
}

fn get_update_progress<'a, M, R>(_req: &mut R, progress: &M) -> Result<ResponseData, serde_json::Error>
where
    M: Mutex<Data = Option<f32>>,
    R: Request<'a>,
{
    ResponseData::from_json(&*progress.lock()).into()
}
