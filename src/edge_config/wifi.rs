use crate::httpd::*;
use crate::httpd::registry::*;
use crate::wifi;

use super::*;

pub struct WifiSession { // Future
}

pub trait AsWifiSession: AsRole {
    fn as_wifi_session(&self) -> &WifiSession;
    fn as_wifi_session_mut(&mut self) -> &mut WifiSession;
}

pub trait AsWifi<W: wifi::Wifi> {
    fn as_wifi(&self) -> &W;
    fn as_wifi_mut(&mut self) -> &mut W;
}

pub fn register<RR: Registry<R, S>, R: Request<S, A>, S, A, W>(registry: RR, prefix: impl AsRef<str>, default_role: Role) -> anyhow::Result<RR>
    where
        A: AsWifi<W> + 'static, W: wifi::Wifi + 'static, S: AsWifiSession + 'static, R: 'static {
    let prefix = |s| [prefix.as_ref(), s].concat();

    registry
        .at(prefix("/scan")).post(with_permissions(default_role, scan))?
        .at(prefix("/conf")).get(with_permissions(default_role, get_configuration))?
        .at(prefix("/conf")).put(with_permissions(default_role, set_configuration))
}

pub fn get_registrations<R: Request<S, A>, S, A, W>(prefix: impl AsRef<str>, default_role: Role) -> std::vec::Vec<Registration<R, S>>
    where
        A: AsWifi<W> + 'static, W: wifi::Wifi + 'static, S: AsWifiSession + 'static, R: 'static {

    let prefix = |s| [prefix.as_ref(), s].concat();

    return vec! [
        Registration::new_post(prefix("/caps"), with_permissions(default_role, get_capabilities)),
        Registration::new_post(prefix(""), with_permissions(default_role, get_status)),
        Registration::new_post(prefix("/scan"), with_permissions(default_role, scan)),
        Registration::new_get(prefix("/conf"), with_permissions(default_role, get_configuration)),
        Registration::new_put(prefix("/conf"), with_permissions(default_role, set_configuration)),
    ]
}

fn get_capabilities<S, A, W>(req: &mut impl Request<S, A>) -> anyhow::Result<Response<S>>
    where
        A: AsWifi<W>, W: wifi::Wifi {
    let caps = req.with_app(|a| a.as_wifi().get_capabilities())?;

    json(&caps)
}

fn get_status<S, A, W>(req: &mut impl Request<S, A>) -> anyhow::Result<Response<S>>
    where
        A: AsWifi<W>, W: wifi::Wifi {
    let status = req.with_app(|a| a.as_wifi().get_status());

    json(&status)
}

fn scan<S, A, W>(req: &mut impl Request<S, A>) -> anyhow::Result<Response<S>>
    where
        A: AsWifi<W>, W: wifi::Wifi {
    let data = req.with_app_mut(|a| a.as_wifi_mut().scan())?;

    json(&data)
}

fn get_configuration<S, A, W>(req: &mut impl Request<S, A>) -> anyhow::Result<Response<S>>
    where
        A: AsWifi<W>, W: wifi::Wifi {
    let conf = req.with_app(|a| a.as_wifi().get_configuration())?;

    json(&conf)
}

fn set_configuration<S, A, W>(req: &mut impl Request<S, A>) -> anyhow::Result<Response<S>>
    where
        A: AsWifi<W>, W: wifi::Wifi {
    let conf: wifi::Configuration = serde_json::from_slice(
        req.as_bytes()?
        .as_slice())?;

    req.with_app_mut(|a| a.as_wifi_mut().set_configuration(&conf))?;

    Ok(().into())
}

fn with_permissions<R: Request<S, A>, S, A>(
        default_role: Role,
        f: impl Fn(&mut R) -> anyhow::Result<Response<S>>) -> impl Fn(&mut R) -> anyhow::Result<Response<S>>
    where
    S: AsRole {
    with_role(Role::Admin, default_role, f)
}

fn json<S, T: ?Sized + serde::Serialize>(data: &T) -> anyhow::Result<Response<S>> {
    Ok(ResponseBuilder::ok()
        .content_type("application/json".to_string())
        .body(serde_json::to_string(data)?.into()).into())
}
