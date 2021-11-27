use core::{cell::RefCell, fmt::Write, str::Split, time::Duration};

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;

use log::*;
use serde::de::DeserializeOwned;

use crate::mutex::*;

use super::*;

pub struct Sessions<M, S>
where
    M: Mutex<Data = BTreeMap<String, Arc<S>>>,
    S: Mutex<Data = SessionState>,
{
    get_random: Box<dyn Fn() -> [u8; 16]>,
    current_time: Box<dyn Fn() -> Duration>,
    max_sessions: usize,
    default_session_timeout: Duration,
    data: M,
}

#[derive(Debug)]
pub enum SessionState {
    Timeout,
    Invalid,
    Ok(SessionData),
}

#[derive(Debug, Default)]
pub struct SessionData {
    last_accessed: Duration,
    timeout: Duration,
    used: u32,
    attributes: BTreeMap<String, Vec<u8>>,
}

pub struct RequestScopedSession<M, S>
where
    M: Mutex<Data = BTreeMap<String, Arc<S>>>,
    S: Mutex<Data = SessionState>,
{
    sessions: Arc<Sessions<M, S>>,
    session_id: Option<String>,
    session: Option<Arc<S>>,
}

impl<M, S> RequestScopedSession<M, S>
where
    M: Mutex<Data = BTreeMap<String, Arc<S>>>,
    S: Mutex<Data = SessionState>,
{
    pub fn new(sessions: Arc<Sessions<M, S>>, session_id: Option<impl AsRef<str>>) -> Self {
        let session = session_id
            .as_ref()
            .map(|session_id| {
                sessions.data.with_lock(|data| {
                    data.get(session_id.as_ref()).map(|s| {
                        s.with_lock(|ss| {
                            if let SessionState::Ok(sd) = ss {
                                sd.used += 1;

                                Some(s.clone())
                            } else {
                                None
                            }
                        })
                    })
                })
            })
            .flatten()
            .flatten();

        Self {
            sessions: sessions.clone(),
            session_id: session_id.map(|s| s.as_ref().to_owned()),
            session,
        }
    }

    fn with_session<Q>(
        &self,
        f: impl Fn(&mut SessionData) -> Result<Q, SessionError>,
    ) -> Result<Q, SessionError> {
        if let Some(session) = self.session.as_ref() {
            session.with_lock(|ss| match ss {
                SessionState::Timeout => Err(SessionError::Timeout),
                SessionState::Invalid => Err(SessionError::Invalidated),
                SessionState::Ok(sd) => f(sd),
            })
        } else {
            Err(SessionError::Missing)
        }
    }

    fn deserialize<T: DeserializeOwned>(
        slice: Option<impl AsRef<[u8]>>,
    ) -> Result<Option<T>, SessionError> {
        if let Some(value) = slice {
            let value = value.as_ref();

            let result = serde_json::from_slice::<T>(value).map_err(|_| SessionError::Serde)?;

            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}

impl<M, S> Drop for RequestScopedSession<M, S>
where
    M: Mutex<Data = BTreeMap<String, Arc<S>>>,
    S: Mutex<Data = SessionState>,
{
    fn drop(&mut self) {
        let now = (self.sessions.current_time)();

        if let Some(session) = self.session.as_ref() {
            session.with_lock(|ss| {
                if let SessionState::Ok(sd) = ss {
                    sd.last_accessed = now;
                    sd.used -= 1;
                }
            });
        }

        self.sessions.cleanup();
    }
}

impl<'a, M, S> Session<'a> for RequestScopedSession<M, S>
where
    M: Mutex<Data = BTreeMap<String, Arc<S>>>,
    S: Mutex<Data = SessionState>,
{
    fn get_error(&self) -> Option<SessionError> {
        self.with_session(|_| Ok(()))
            .map_or_else(Option::Some, |_| None)
    }

    fn create_if_invalid(&mut self) -> Result<&mut Self, SessionError> {
        let valid = if let Some(session) = self.session.as_ref() {
            session.with_lock(|ss| matches!(ss, SessionState::Ok(_)))
        } else {
            false
        };

        if !valid {
            let session_id = self.sessions.generate_session_id();
            let last_accessed = (self.sessions.current_time)();
            let timeout = self.sessions.default_session_timeout;

            let session = Arc::new(S::new(SessionState::Ok(SessionData {
                last_accessed,
                timeout,
                ..Default::default()
            })));

            self.sessions.data.with_lock(|data| {
                if data.len() < self.sessions.max_sessions {
                    data.insert(session_id.clone(), session.clone());

                    Ok(())
                } else {
                    Err(SessionError::MaxSessiuonsReached)
                }
            })?;

            self.session_id = Some(session_id);
            self.session = Some(session);
        }

        Ok(self)
    }

    fn get<T: DeserializeOwned>(&self, name: impl AsRef<str>) -> Result<Option<T>, SessionError> {
        self.with_session(|sd| Self::deserialize(sd.attributes.get(name.as_ref())))
    }

    fn set_and_get<I: Serialize, T: DeserializeOwned>(
        &mut self,
        name: impl AsRef<str>,
        value: &I,
    ) -> Result<Option<T>, SessionError> {
        self.with_session(|sd| {
            Self::deserialize(sd.attributes.insert(
                name.as_ref().to_owned(),
                serde_json::to_vec(value).map_err(|_| SessionError::Serde)?,
            ))
        })
    }

    fn remove_and_get<T: DeserializeOwned>(
        &mut self,
        name: impl AsRef<str>,
    ) -> Result<Option<T>, SessionError> {
        self.with_session(|sd| Self::deserialize(sd.attributes.remove(name.as_ref())))
    }

    fn set<I: Serialize>(
        &mut self,
        name: impl AsRef<str>,
        value: &I,
    ) -> Result<bool, SessionError> {
        self.with_session(|sd| {
            Ok(sd
                .attributes
                .insert(
                    name.as_ref().to_owned(),
                    serde_json::to_vec(value).map_err(|_| SessionError::Serde)?,
                )
                .is_some())
        })
    }

    fn remove(&mut self, name: impl AsRef<str>) -> Result<bool, SessionError> {
        self.with_session(|sd| Ok(sd.attributes.remove(name.as_ref()).is_some()))
    }

    fn invalidate(&mut self) -> Result<bool, SessionError> {
        let valid = self.sessions.data.with_lock(|data| {
            if let Some(session) = self.session.as_ref() {
                session.with_lock(|ss| {
                    if matches!(ss, SessionState::Ok(_)) {
                        *ss = SessionState::Invalid;

                        data.remove(self.session_id.as_ref().unwrap());

                        true
                    } else {
                        false
                    }
                })
            } else {
                false
            }
        });

        Ok(valid)
    }
}

pub struct RequestScopedSessionReference<'a, M, S>(&'a RefCell<RequestScopedSession<M, S>>)
where
    M: Mutex<Data = BTreeMap<String, Arc<S>>>,
    S: Mutex<Data = SessionState>;

impl<'a, M, S> Session<'a> for RequestScopedSessionReference<'a, M, S>
where
    M: Mutex<Data = BTreeMap<String, Arc<S>>>,
    S: Mutex<Data = SessionState>,
{
    fn get_error(&self) -> Option<SessionError> {
        self.0.borrow().get_error()
    }

    fn create_if_invalid(&mut self) -> Result<&mut Self, SessionError> {
        self.0.borrow_mut().create_if_invalid()?;

        Ok(self)
    }

    fn get<T: DeserializeOwned>(&self, name: impl AsRef<str>) -> Result<Option<T>, SessionError> {
        self.0.borrow().get(name)
    }

    fn set_and_get<I: Serialize, T: DeserializeOwned>(
        &mut self,
        name: impl AsRef<str>,
        value: &I,
    ) -> Result<Option<T>, SessionError> {
        self.0.borrow_mut().set_and_get(name, value)
    }

    fn remove_and_get<T: DeserializeOwned>(
        &mut self,
        name: impl AsRef<str>,
    ) -> Result<Option<T>, SessionError> {
        self.0.borrow_mut().remove_and_get(name)
    }

    fn set<I: Serialize>(
        &mut self,
        name: impl AsRef<str>,
        value: &I,
    ) -> Result<bool, SessionError> {
        self.0.borrow_mut().set(name, value)
    }

    fn remove(&mut self, name: impl AsRef<str>) -> Result<bool, SessionError> {
        self.0.borrow_mut().remove(name)
    }

    fn invalidate(&mut self) -> Result<bool, SessionError> {
        self.0.borrow_mut().invalidate()
    }
}

impl<M, S> Sessions<M, S>
where
    M: Mutex<Data = BTreeMap<String, Arc<S>>>,
    S: Mutex<Data = SessionState>,
{
    pub fn new(
        get_random: impl Fn() -> [u8; 16] + 'static,
        current_time: impl Fn() -> Duration + 'static,
        max_sessions: usize,
        default_session_timeout: Duration,
    ) -> Self {
        Self {
            get_random: Box::new(get_random),
            current_time: Box::new(current_time),
            max_sessions,
            default_session_timeout,
            data: M::new(BTreeMap::new()),
        }
    }

    pub fn get_session_id(cookies: Option<impl AsRef<str>>) -> Option<String> {
        cookies.and_then(|s| Self::parse_session_cookie(s.as_ref()).map(str::to_owned))
    }

    pub fn insert_session_cookie(
        cookies: Option<impl AsRef<str>>,
        session_id: impl AsRef<str>,
    ) -> String {
        let cookies = cookies
            .as_ref()
            .map(|cookies| cookies.as_ref())
            .unwrap_or("");

        CookieIterator::collect(
            CookieIterator::new(cookies)
                .filter(|(name, _)| *name != "SESSIONID")
                .chain(core::iter::once(("SESSIONID", session_id.as_ref()))),
        )
    }

    fn parse_session_cookie<'a>(cookies: &'a str) -> Option<&'a str> {
        CookieIterator::new(cookies)
            .find(|(name, _)| *name == "SESSIONID")
            .map(|(_, value)| value)
    }

    fn generate_session_id(&self) -> String {
        let new_session_id_bytes = (self.get_random)();

        let mut new_session_id = String::new();

        struct ByteBuf<'a>(&'a [u8]);

        impl<'a> core::fmt::LowerHex for ByteBuf<'a> {
            fn fmt(&self, fmtr: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                for byte in self.0 {
                    fmtr.write_fmt(format_args!("{:02x}", byte))?;
                }

                Ok(())
            }
        }

        write!(&mut new_session_id, "{:x}", ByteBuf(&new_session_id_bytes))
            .expect("Unable to write");

        new_session_id
    }

    fn cleanup(&self) {
        info!("Performing sessions cleanup");

        let now = (self.current_time)();

        self.data.with_lock(|data| {
            data.retain(|_, v| {
                v.with_lock(|ss| {
                    if let SessionState::Ok(sd) = ss {
                        if sd.used == 0 && now - sd.last_accessed > sd.timeout {
                            *ss = SessionState::Timeout;

                            false
                        } else {
                            true
                        }
                    } else {
                        false
                    }
                })
            });
        });
    }
}
struct CookieIterator<'a>(Split<'a, char>);

impl<'a> CookieIterator<'a> {
    pub fn new(cookies: &'a str) -> Self {
        Self(cookies.split(';'))
    }

    pub fn collect<'b>(iter: impl Iterator<Item = (&'b str, &'b str)>) -> String {
        let mut result = String::new();
        for (key, value) in iter {
            if result.len() > 0 {
                result.push(';');
            }

            result.push_str(key);
            result.push('=');
            result.push_str(value);
        }

        result
    }
}

impl<'a> Iterator for CookieIterator<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|cookie_pair| cookie_pair.split('='))
            .and_then(|mut cookie_pair| {
                cookie_pair
                    .next()
                    .map(|name| cookie_pair.next().map(|value| (name, value)))
            })
            .flatten()
    }
}
