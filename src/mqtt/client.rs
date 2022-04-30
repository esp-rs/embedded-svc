use core::fmt::{self, Debug, Display, Formatter};

extern crate alloc;
use alloc::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::errors::Errors;

/// Quality of service
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

pub type MessageId = u32;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Event<M> {
    BeforeConnect,
    Connected(bool),
    Disconnected,
    Subscribed(MessageId),
    Unsubscribed(MessageId),
    Published(MessageId),
    Received(M),
    Deleted(MessageId),
}

impl<M> Event<M> {
    pub fn transform_received<F, O>(&self, f: F) -> Event<O>
    where
        F: FnOnce(&M) -> O,
    {
        match self {
            Self::Received(message) => Event::Received(f(message)),
            Self::BeforeConnect => Event::BeforeConnect,
            Self::Connected(connected) => Event::Connected(*connected),
            Self::Disconnected => Event::Disconnected,
            Self::Subscribed(message_id) => Event::Subscribed(*message_id),
            Self::Unsubscribed(message_id) => Event::Unsubscribed(*message_id),
            Self::Published(message_id) => Event::Published(*message_id),
            Self::Deleted(message_id) => Event::Deleted(*message_id),
        }
    }
}

impl<M> Display for Event<M>
where
    M: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::BeforeConnect => write!(f, "BeforeConnect"),
            Self::Connected(connected) => write!(f, "Connected(session: {})", connected),
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Subscribed(message_id) => write!(f, "Subscribed({})", message_id),
            Self::Unsubscribed(message_id) => write!(f, "Unsubscribed({})", message_id),
            Self::Published(message_id) => write!(f, "Published({})", message_id),
            Self::Received(message) => write!(f, "Received({})", message),
            Self::Deleted(message_id) => write!(f, "Deleted({})", message_id),
        }
    }
}

pub trait Message {
    fn id(&self) -> MessageId;

    fn topic(&self) -> Option<Cow<'_, str>>;

    fn data(&self) -> Cow<'_, [u8]>;

    fn details(&self) -> &Details;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageImpl {
    id: MessageId,
    topic: Option<String>,
    details: Details,
    data: Vec<u8>,
}

impl MessageImpl {
    pub fn new<M>(message: &M) -> Self
    where
        M: Message,
    {
        Self {
            id: message.id(),
            data: message.data().to_vec(),
            topic: message.topic().map(|topic| topic.into_owned()),
            details: message.details().clone(),
        }
    }
}

impl Message for MessageImpl {
    fn id(&self) -> MessageId {
        self.id
    }

    fn topic(&self) -> Option<Cow<'_, str>> {
        self.topic
            .as_ref()
            .map(|topic| Cow::Borrowed(topic.as_str()))
    }

    fn data(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(&self.data)
    }

    fn details(&self) -> &Details {
        &self.details
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Details {
    Complete,
    InitialChunk(InitialChunkData),
    SubsequentChunk(SubsequentChunkData),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitialChunkData {
    pub total_data_size: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubsequentChunkData {
    pub current_data_offset: usize,
    pub total_data_size: usize,
}

pub trait Client: Errors {
    fn subscribe<'a, S>(&'a mut self, topic: S, qos: QoS) -> Result<MessageId, Self::Error>
    where
        S: Into<Cow<'a, str>>;

    fn unsubscribe<'a, S>(&'a mut self, topic: S) -> Result<MessageId, Self::Error>
    where
        S: Into<Cow<'a, str>>;
}

impl<'b, C> Client for &'b mut C
where
    C: Client,
{
    fn subscribe<'a, S>(&'a mut self, topic: S, qos: QoS) -> Result<MessageId, Self::Error>
    where
        S: Into<Cow<'a, str>>,
    {
        (*self).subscribe(topic, qos)
    }

    fn unsubscribe<'a, S>(&'a mut self, topic: S) -> Result<MessageId, Self::Error>
    where
        S: Into<Cow<'a, str>>,
    {
        (*self).unsubscribe(topic)
    }
}

pub trait Publish: Errors {
    fn publish<'a, S, V>(
        &'a mut self,
        topic: S,
        qos: QoS,
        retain: bool,
        payload: V,
    ) -> Result<MessageId, Self::Error>
    where
        S: Into<Cow<'a, str>>,
        V: Into<Cow<'a, [u8]>>;
}

impl<'b, P> Publish for &'b mut P
where
    P: Publish,
{
    fn publish<'a, S, V>(
        &'a mut self,
        topic: S,
        qos: QoS,
        retain: bool,
        payload: V,
    ) -> Result<MessageId, Self::Error>
    where
        S: Into<Cow<'a, str>>,
        V: Into<Cow<'a, [u8]>>,
    {
        (*self).publish(topic, qos, retain, payload)
    }
}

pub trait Enqueue: Errors {
    fn enqueue<'a, S, V>(
        &'a mut self,
        topic: S,
        qos: QoS,
        retain: bool,
        payload: V,
    ) -> Result<MessageId, Self::Error>
    where
        S: Into<Cow<'a, str>>,
        V: Into<Cow<'a, [u8]>>;
}

impl<'b, E> Enqueue for &'b mut E
where
    E: Enqueue,
{
    fn enqueue<'a, S, V>(
        &'a mut self,
        topic: S,
        qos: QoS,
        retain: bool,
        payload: V,
    ) -> Result<MessageId, Self::Error>
    where
        S: Into<Cow<'a, str>>,
        V: Into<Cow<'a, [u8]>>,
    {
        (*self).enqueue(topic, qos, retain, payload)
    }
}

pub trait Connection: Errors {
    type Message;

    fn next(&mut self) -> Option<Result<Event<Self::Message>, Self::Error>>;
}

impl<'b, C> Connection for &'b mut C
where
    C: Connection,
{
    type Message = C::Message;

    fn next(&mut self) -> Option<Result<Event<Self::Message>, Self::Error>> {
        (*self).next()
    }
}

#[cfg(feature = "alloc")]
pub mod utils {
    use core::mem;

    use alloc::sync::Arc;

    use crate::{
        errors,
        mutex::{Condvar, Mutex},
    };

    use super::Event;

    pub struct ConnStateGuard<CV, S>
    where
        CV: Condvar,
    {
        pub state: CV::Mutex<Option<S>>,
        pub state_changed: CV,
    }

    impl<CV, S> ConnStateGuard<CV, S>
    where
        CV: Condvar,
    {
        pub fn new(state: S) -> Self {
            Self {
                state: CV::Mutex::new(Some(state)),
                state_changed: CV::new(),
            }
        }
    }

    impl<CV, S> ConnStateGuard<CV, S>
    where
        CV: Condvar,
        S: Default,
    {
        pub fn new_default() -> Self {
            Self::new(Default::default())
        }
    }

    impl<CV, S> ConnStateGuard<CV, S>
    where
        CV: Condvar,
    {
        pub fn close(&self) {
            let mut state = self.state.lock();

            *state = None;
            self.state_changed.notify_all();
        }
    }

    impl<CV, S> Default for ConnStateGuard<CV, S>
    where
        CV: Condvar,
        S: Default,
    {
        fn default() -> Self {
            Self::new(Default::default())
        }
    }

    pub struct ConnState<M, E>(Option<Result<Event<M>, E>>);

    impl<M, E> Default for ConnState<M, E> {
        fn default() -> Self {
            Self(Default::default())
        }
    }

    pub struct Postbox<CV, M, E>(Arc<ConnStateGuard<CV, ConnState<M, E>>>)
    where
        CV: Condvar;

    impl<CV, M, E> Postbox<CV, M, E>
    where
        CV: Condvar,
    {
        pub fn new(connection_state: Arc<ConnStateGuard<CV, ConnState<M, E>>>) -> Self {
            Self(connection_state)
        }

        pub fn post(&mut self, event: Result<Event<M>, E>) {
            let mut state = self.0.state.lock();

            loop {
                if let Some(data) = &mut *state {
                    if data.0.is_some() {
                        state = self.0.state_changed.wait(state);
                    } else {
                        break;
                    }
                } else {
                    return;
                }
            }

            *state = Some(ConnState(Some(event)));
            self.0.state_changed.notify_all();
        }
    }

    pub struct Connection<CV, M, E>(Arc<ConnStateGuard<CV, ConnState<M, E>>>)
    where
        CV: Condvar;

    impl<CV, M, E> Connection<CV, M, E>
    where
        CV: Condvar,
        E: errors::Error,
    {
        pub fn new(connection_state: Arc<ConnStateGuard<CV, ConnState<M, E>>>) -> Self {
            Self(connection_state)
        }
    }

    impl<CV, M, E> errors::Errors for Connection<CV, M, E>
    where
        CV: Condvar,
        E: errors::Error,
    {
        type Error = E;
    }

    impl<CV, M, E> super::Connection for Connection<CV, M, E>
    where
        CV: Condvar,
        E: errors::Error,
    {
        type Message = M;

        fn next(&mut self) -> Option<Result<Event<Self::Message>, Self::Error>> {
            let mut state = self.0.state.lock();

            loop {
                if let Some(data) = &mut *state {
                    let pulled = mem::replace(data, ConnState(None));

                    match pulled {
                        ConnState(Some(event)) => {
                            self.0.state_changed.notify_all();
                            return Some(event);
                        }
                        ConnState(None) => state = self.0.state_changed.wait(state),
                    }
                } else {
                    return None;
                }
            }
        }
    }
}

#[cfg(feature = "experimental")]
pub mod asyncs {
    use core::future::Future;

    extern crate alloc;
    use alloc::borrow::Cow;

    pub use super::{Details, Event, Message, MessageId, QoS};

    use crate::errors::Errors;

    pub trait Client: Errors {
        type SubscribeFuture<'a>: Future<Output = Result<MessageId, Self::Error>> + Send
        where
            Self: 'a;
        type UnsubscribeFuture<'a>: Future<Output = Result<MessageId, Self::Error>> + Send
        where
            Self: 'a;

        fn subscribe<'a, S>(&'a mut self, topic: S, qos: QoS) -> Self::SubscribeFuture<'a>
        where
            S: Into<Cow<'a, str>>;

        fn unsubscribe<'a, S>(&'a mut self, topic: S) -> Self::UnsubscribeFuture<'a>
        where
            S: Into<Cow<'a, str>>;
    }

    pub trait Publish: Errors {
        type PublishFuture<'a>: Future<Output = Result<MessageId, Self::Error>> + Send
        where
            Self: 'a;

        fn publish<'a, S, V>(
            &'a mut self,
            topic: S,
            qos: QoS,
            retain: bool,
            payload: V,
        ) -> Self::PublishFuture<'a>
        where
            S: Into<Cow<'a, str>>,
            V: Into<Cow<'a, [u8]>>;
    }

    /// core.stream.Stream is not stable yet and on top of that it has an Item which is not
    /// parameterizable by lifetime (GATs). Therefore, we have to use a Future instead
    pub trait Connection: Errors {
        type Message;

        type NextFuture<'a>: Future<Output = Option<Result<Event<Self::Message>, Self::Error>>>
            + Send
        where
            Self: 'a;

        fn next(&mut self) -> Self::NextFuture<'_>;
    }
}
