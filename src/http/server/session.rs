use core::{cell::RefCell, fmt::Write, time::Duration};

extern crate alloc;
use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;

use log::*;

use crate::mutex::*;

use super::*;

pub struct Sessions<M, S>
where
    M: Mutex<Data = BTreeMap<String, SessionData<S>>>,
    S: Mutex<Data = Option<BTreeMap<String, Vec<u8>>>>,
{
    get_random: Box<dyn Fn() -> [u8; 16]>,
    current_time: Box<dyn Fn() -> Duration>,
    max_sessions: usize,
    default_session_timeout: Duration,
    data: M,
}

#[derive(Debug, Default)]
pub struct SessionData<S>
where
    S: Mutex<Data = Option<BTreeMap<String, Vec<u8>>>>,
{
    last_accessed: Duration,
    timeout: Duration,
    used: u32,
    state: Arc<S>,
}

pub struct RequestScopedSession<M, S>
where
    M: Mutex<Data = BTreeMap<String, SessionData<S>>>,
    S: Mutex<Data = Option<BTreeMap<String, Vec<u8>>>>,
{
    sessions: Arc<Sessions<M, S>>,
    session_id: Option<String>,
    session: Option<Arc<S>>,
}

impl<M, S> RequestScopedSession<M, S>
where
    M: Mutex<Data = BTreeMap<String, SessionData<S>>>,
    S: Mutex<Data = Option<BTreeMap<String, Vec<u8>>>>,
{
    pub fn new(sessions: Arc<Sessions<M, S>>, session_id: Option<impl AsRef<str>>) -> Self {
        let session = session_id.as_ref().and_then(|session_id| {
            let mut data = sessions.data.lock();

            let sd = data.get_mut(session_id.as_ref());

            if let Some(sd) = sd {
                sd.used += 1;
                Some(sd.state.clone())
            } else {
                None
            }
        });

        Self {
            sessions,
            session_id: if session.is_some() {
                session_id.map(|s| s.as_ref().to_owned())
            } else {
                None
            },
            session,
        }
    }

    fn create(&mut self) -> Result<(), SessionError> {
        let now = (self.sessions.current_time)();
        let session_id = self.sessions.generate_session_id();
        let max_sessions = self.sessions.max_sessions;

        let mut sessions = self.sessions.data.lock();

        if sessions.len() >= max_sessions {
            return Err(SessionError::MaxSessiuonsReachedError);
        }

        let session = Arc::new(S::new(Some(BTreeMap::new())));

        let sd = SessionData {
            last_accessed: now,
            timeout: self.sessions.default_session_timeout,
            used: 1,
            state: session.clone(),
        };

        sessions.insert(session_id.clone(), sd);

        self.session_id = Some(session_id);
        self.session = Some(session);

        Ok(())
    }

    fn release(&mut self, only_if_invalid: bool) -> (Option<String>, bool) {
        let result = if let Some(session) = self.session.as_ref() {
            let now = (self.sessions.current_time)();

            let ss = session.lock();

            let valid = ss.is_some();
            if only_if_invalid && valid {
                return (None, false);
            }

            let session_id = self.session_id.as_ref().unwrap().clone();

            let mut sessions = self.sessions.data.lock();

            let sd = sessions.get_mut(&session_id);

            if let Some(sd) = sd {
                sd.used -= 1;
                sd.last_accessed = now;

                if sd.used == 0 && !valid {
                    sessions.remove(&session_id);
                }
            } else if valid {
                let sd = SessionData {
                    last_accessed: now,
                    timeout: self.sessions.default_session_timeout,
                    used: 0,
                    state: session.clone(),
                };

                sessions.insert(session_id.clone(), sd);
            }

            (Some(session_id), true)
        } else {
            (None, true)
        };

        self.session = None;
        self.session_id = None;

        result
    }

    fn with_session<Q>(
        &self,
        f: impl Fn(&mut BTreeMap<String, Vec<u8>>) -> Result<Q, SessionError>,
    ) -> Result<Q, SessionError> {
        if let Some(session) = self.session.as_ref() {
            let ss = &mut *session.lock();
            match ss {
                None => Err(SessionError::InvalidatedError),
                Some(attrs) => f(attrs),
            }
        } else {
            Err(SessionError::MissingError)
        }
    }

    fn deserialize<T: serde::de::DeserializeOwned>(
        slice: Option<impl AsRef<[u8]>>,
    ) -> Result<Option<T>, SessionError> {
        if let Some(value) = slice {
            let value = value.as_ref();

            let result =
                serde_json::from_slice::<T>(value).map_err(|_| SessionError::SerdeError)?;

            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}

impl<M, S> Drop for RequestScopedSession<M, S>
where
    M: Mutex<Data = BTreeMap<String, SessionData<S>>>,
    S: Mutex<Data = Option<BTreeMap<String, Vec<u8>>>>,
{
    fn drop(&mut self) {
        let _ = self.release(false);

        self.sessions.cleanup();
    }
}

impl<'a, M, S> Session<'a> for RequestScopedSession<M, S>
where
    M: Mutex<Data = BTreeMap<String, SessionData<S>>>,
    S: Mutex<Data = Option<BTreeMap<String, Vec<u8>>>>,
{
    fn get_error(&self) -> Option<SessionError> {
        self.with_session(|_| Ok(()))
            .map_or_else(Option::Some, |_| None)
    }

    fn id(&self) -> Option<Cow<'_, str>> {
        self.session_id
            .as_ref()
            .map(|session_id| Cow::Borrowed(session_id.as_ref()))
    }

    fn create_if_invalid(&mut self) -> Result<&mut Self, SessionError> {
        let (_, released) = self.release(true);

        if released {
            self.create()?;
        }

        Ok(self)
    }

    fn get<T: serde::de::DeserializeOwned>(&self, name: impl AsRef<str>) -> Result<Option<T>, SessionError> {
        self.with_session(|attributes| Self::deserialize(attributes.get(name.as_ref())))
    }

    fn set_and_get<I: serde::Serialize, T: serde::de::DeserializeOwned>(
        &mut self,
        name: impl AsRef<str>,
        value: &I,
    ) -> Result<Option<T>, SessionError> {
        self.with_session(|attributes| {
            Self::deserialize(attributes.insert(
                name.as_ref().to_owned(),
                serde_json::to_vec(value).map_err(|_| SessionError::SerdeError)?,
            ))
        })
    }

    fn remove_and_get<T: serde::de::DeserializeOwned>(
        &mut self,
        name: impl AsRef<str>,
    ) -> Result<Option<T>, SessionError> {
        self.with_session(|attributes| Self::deserialize(attributes.remove(name.as_ref())))
    }

    fn set<I: serde::Serialize>(
        &mut self,
        name: impl AsRef<str>,
        value: &I,
    ) -> Result<bool, SessionError> {
        self.with_session(|attributes| {
            Ok(attributes
                .insert(
                    name.as_ref().to_owned(),
                    serde_json::to_vec(value).map_err(|_| SessionError::SerdeError)?,
                )
                .is_some())
        })
    }

    fn remove(&mut self, name: impl AsRef<str>) -> Result<bool, SessionError> {
        self.with_session(|attributes| Ok(attributes.remove(name.as_ref()).is_some()))
    }

    fn invalidate(&mut self) -> Result<bool, SessionError> {
        let mut data = self.sessions.data.lock();

        let valid = if let Some(session) = self.session.as_ref() {
            let mut ss = session.lock();

            if ss.is_some() {
                *ss = None;

                data.remove(self.session_id.as_ref().unwrap());

                true
            } else {
                false
            }
        } else {
            false
        };

        Ok(valid)
    }
}

pub struct RequestScopedSessionReference<'a, M, S>(&'a RefCell<RequestScopedSession<M, S>>)
where
    M: Mutex<Data = BTreeMap<String, SessionData<S>>>,
    S: Mutex<Data = Option<BTreeMap<String, Vec<u8>>>>;

impl<'a, M, S> RequestScopedSessionReference<'a, M, S>
where
    M: Mutex<Data = BTreeMap<String, SessionData<S>>>,
    S: Mutex<Data = Option<BTreeMap<String, Vec<u8>>>>,
{
    pub fn new(session: &'a RefCell<RequestScopedSession<M, S>>) -> Self {
        Self(session)
    }
}

impl<'a, M, S> Session<'a> for RequestScopedSessionReference<'a, M, S>
where
    M: Mutex<Data = BTreeMap<String, SessionData<S>>>,
    S: Mutex<Data = Option<BTreeMap<String, Vec<u8>>>>,
{
    fn get_error(&self) -> Option<SessionError> {
        self.0.borrow().get_error()
    }

    fn id(&self) -> Option<Cow<'_, str>> {
        self.0
            .borrow()
            .id()
            .map(|value| Cow::Owned(value.into_owned())) // TODO
    }

    fn create_if_invalid(&mut self) -> Result<&mut Self, SessionError> {
        self.0.borrow_mut().create_if_invalid()?;

        Ok(self)
    }

    fn get<T: serde::de::DeserializeOwned>(&self, name: impl AsRef<str>) -> Result<Option<T>, SessionError> {
        self.0.borrow().get(name)
    }

    fn set_and_get<I: serde::Serialize, T: serde::de::DeserializeOwned>(
        &mut self,
        name: impl AsRef<str>,
        value: &I,
    ) -> Result<Option<T>, SessionError> {
        self.0.borrow_mut().set_and_get(name, value)
    }

    fn remove_and_get<T: serde::de::DeserializeOwned>(
        &mut self,
        name: impl AsRef<str>,
    ) -> Result<Option<T>, SessionError> {
        self.0.borrow_mut().remove_and_get(name)
    }

    fn set<I: serde::Serialize>(
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
    M: Mutex<Data = BTreeMap<String, SessionData<S>>>,
    S: Mutex<Data = Option<BTreeMap<String, Vec<u8>>>>,
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

    fn generate_session_id(&self) -> String {
        let new_session_id_bytes = (self.get_random)();

        let mut new_session_id = String::new();

        struct ByteBuf<'a>(&'a [u8]);

        impl<'a> core::fmt::LowerHex for ByteBuf<'a> {
            fn fmt(&self, fmtr: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
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
        info!("Performing stale sessions cleanup");

        let now = (self.current_time)();

        self.data
            .lock()
            .retain(|_, sd| sd.used > 0 || now - sd.last_accessed < sd.timeout);
    }
}
