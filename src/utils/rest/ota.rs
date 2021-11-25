extern crate alloc;
use alloc::sync::Arc;

use super::role::Role;

use crate::{
    httpd::registry::*,
    httpd::*,
    mutex::*,
    ota::{self, OtaRead, OtaSlot, OtaUpdate},
};

use super::*;

pub fn register<R, MO, MS, MP, O, S>(
    registry: R,
    pref: &str,
    ota: Arc<MO>,
    ota_server: Arc<MS>,
    progress: Arc<MP>,
    default_role: Option<Role>,
) -> Result<R>
where
    R: Registry,
    MO: Mutex<Data = O> + 'static,
    MS: Mutex<Data = S> + 'static,
    MP: Mutex<Data = Option<f32>> + 'static,
    O: ota::Ota,
    S: ota::OtaServer,
{
    let prefix = |s| [pref, s].concat();

    let otas_get_updates = ota_server.clone();
    let otas_get_latest_update = ota_server.clone();
    let otas_update = ota_server.clone();
    let ota_get_status = ota.clone();
    let ota_factory_reset = ota.clone();
    let progress_update = progress.clone();

    registry
        .at(prefix(""))
        .get(move |req| get_status(req, &*ota_get_status))?
        .at(prefix("/updates"))
        .get(move |req| get_updates(req, &*otas_get_updates))?
        .at(prefix("/updates/latest"))
        .get(move |req| get_latest_update(req, &*otas_get_latest_update))?
        .at(prefix("/reset"))
        .post(move |req| factory_reset(req, &*ota_factory_reset))?
        .at(prefix("/update"))
        .put(move |req| update(req, &*ota, &*otas_update, &*progress_update))?
        .at(prefix("/update/progress"))
        .get(move |req| get_update_progress(req, &*progress))?
        .at(pref)
        .middleware(with_permissions(default_role))
}

fn get_status<M, O>(_req: Request, ota: &M) -> Result<Response>
where
    M: Mutex<Data = O>,
    O: ota::Ota,
{
    let info = ota.with_lock(|ota| {
        ota.get_running_slot()
            .map_err(|e| anyhow::anyhow!(e))
            .and_then(|slot| slot.get_firmware_info().map_err(|e| anyhow::anyhow!(e)))
    })?;

    json(&info)
}

fn get_updates<M, O>(_req: Request, ota_server: &M) -> Result<Response>
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

    json(&updates)
}

fn get_latest_update<M, O>(_req: Request, ota_server: &M) -> Result<Response>
where
    M: Mutex<Data = O>,
    O: ota::OtaServer,
{
    let update = ota_server.with_lock(|ota_server| {
        ota_server
            .get_latest_release()
            .map_err(|e| anyhow::anyhow!(e))
    })?;

    json(&update)
}

fn factory_reset<M, O>(_req: Request, ota: &M) -> Result<Response>
where
    M: Mutex<Data = O>,
    O: ota::Ota,
{
    ota.with_lock(|ota| ota.factory_reset().map_err(|e| anyhow::anyhow!(e)))?;

    Ok(Response::ok())
}

fn update<MO, MS, MP, O, S>(
    mut req: Request,
    ota: &MO,
    ota_server: &MS,
    progress: &MP,
) -> Result<Response>
where
    MO: Mutex<Data = O>,
    MS: Mutex<Data = S>,
    MP: Mutex<Data = Option<f32>>,
    O: ota::Ota,
    S: ota::OtaServer,
{
    let download_id: Option<String> = serde_json::from_slice(req.as_bytes()?.as_slice())?;

    ota_server.with_lock(|ota_server| {
        let download_id = match download_id {
            None => ota_server
                .get_latest_release()
                .map_err(|e| anyhow::anyhow!(e))?
                .and_then(|release| release.download_id),
            some => some,
        };

        let download_id = download_id.ok_or_else(|| anyhow::anyhow!("No update"))?;

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
                .map_err(|e| anyhow::anyhow!(e))
        })
    })?;

    Ok(().into())
}

fn get_update_progress<M>(_req: Request, progress: &M) -> Result<Response>
where
    M: Mutex<Data = Option<f32>>,
{
    json(&progress.with_lock(|progress| *progress))
}

fn with_permissions(
    default_role: Option<Role>,
) -> impl for<'r> Fn(Request, &'r dyn Fn(Request) -> Result<Response>) -> Result<Response> {
    auth::with_role(Role::Admin, default_role)
}

fn json<T: ?Sized + serde::Serialize>(data: &T) -> Result<Response> {
    Response::ok()
        .content_type("application/json".to_string())
        .body(serde_json::to_string(data)?.into())
        .into()
}
