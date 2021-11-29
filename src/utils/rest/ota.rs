extern crate alloc;
use alloc::sync::Arc;

use anyhow::{anyhow, Result};

use crate::{
    http::server::registry::*,
    http::server::*,
    io,
    mutex::*,
    ota::{self, OtaRead, OtaSlot, OtaUpdate},
};

use super::{role::Role, *};

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
        .with_middleware(auth::WithRoleMiddleware {
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

fn get_status<'a, M, O>(_req: &mut impl Request<'a>, ota: &M) -> Result<ResponseData>
where
    M: Mutex<Data = O>,
    O: ota::Ota,
{
    let info = ota.with_lock(|ota| {
        ota.get_running_slot()
            .map_err(|e| anyhow::anyhow!(e))
            .and_then(|slot| slot.get_firmware_info().map_err(|e| anyhow::anyhow!(e)))
    })?;

    ResponseData::from_json(&info)?.into()
}

fn get_updates<'a, M, O>(_req: &mut impl Request<'a>, ota_server: &M) -> Result<ResponseData>
where
    M: Mutex<Data = O>,
    O: ota::OtaServer,
{
    // TODO: Not efficient
    let updates = ota_server.with_lock(|ota_server| {
        ota_server
            .get_releases()
            .map(|releases| releases.collect::<Vec<_>>())
            .map_err(|e| anyhow::anyhow!(e))
    })?;

    ResponseData::from_json(&updates)?.into()
}

fn get_latest_update<'a, M, O>(_req: &mut impl Request<'a>, ota_server: &M) -> Result<ResponseData>
where
    M: Mutex<Data = O>,
    O: ota::OtaServer,
{
    let update = ota_server.with_lock(|ota_server| {
        ota_server
            .get_latest_release()
            .map_err(|e| anyhow::anyhow!(e))
    })?;

    ResponseData::from_json(&update)?.into()
}

fn factory_reset<'a, M, O>(_req: &mut impl Request<'a>, ota: &M) -> Result<ResponseData>
where
    M: Mutex<Data = O>,
    O: ota::Ota,
{
    ota.with_lock(|ota| ota.factory_reset().map_err(|e| anyhow::anyhow!(e)))?;

    Ok(ResponseData::ok())
}

fn update<'a, MO, MS, MP, O, S>(
    req: &mut impl Request<'a>,
    ota: &MO,
    ota_server: &MS,
    progress: &MP,
) -> Result<ResponseData>
where
    MO: Mutex<Data = O>,
    MS: Mutex<Data = S>,
    MP: Mutex<Data = Option<f32>>,
    O: ota::Ota,
    S: ota::OtaServer,
{
    let bytes: Result<Vec<_>, _> = io::Bytes::<_, 64>::new(req.reader()).take(3000).collect();

    let bytes = bytes?;

    let download_id = url::form_urlencoded::parse(&bytes)
        .find(|(key, _)| key.as_ref() == "download_id")
        .map(|(_, value)| value.into_owned());

    ota_server.with_lock(|ota_server| {
        let download_id = match download_id {
            None => ota_server
                .get_latest_release()
                .map_err(|e| anyhow::anyhow!(e))?
                .and_then(|release| release.download_id),
            some => some,
        };

        let download_id = download_id.ok_or_else(|| anyhow!("No update"))?;

        let mut ota_update = ota_server
            .open(download_id)
            .map_err(|e| anyhow::anyhow!(e))?;
        let size = ota_update.size();

        ota.with_lock(|ota| {
            ota.initiate_update()
                .map_err(|e| anyhow::anyhow!(e))?
                .update(&mut ota_update, |_, copied| {
                    progress.with_lock(|progress| {
                        *progress = size.map(|size| copied as f32 / size as f32)
                    })
                }) // TODO: Take the progress mutex more rarely
                .map_err(|e| anyhow!(e))
        })
    })?;

    Ok(().into())
}

fn get_update_progress<'a, M>(_req: &mut impl Request<'a>, progress: &M) -> Result<ResponseData>
where
    M: Mutex<Data = Option<f32>>,
{
    ResponseData::from_json(&progress.with_lock(|progress| *progress))?.into()
}
