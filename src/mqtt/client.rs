use core::fmt::{self, Debug, Display, Formatter};

#[cfg(feature = "alloc")]
extern crate alloc;

use serde::{Deserialize, Serialize};

pub trait ErrorType {
    type Error: Debug;
}

impl<E> ErrorType for &E
where
    E: ErrorType,
{
    type Error = E::Error;
}

impl<E> ErrorType for &mut E
where
    E: ErrorType,
{
    type Error = E::Error;
}

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

    fn topic(&self) -> Option<&'_ str>;

    fn data(&self) -> &'_ [u8];

    fn details(&self) -> &Details;
}

impl<M> Message for &M
where
    M: Message,
{
    fn id(&self) -> MessageId {
        (*self).id()
    }

    fn topic(&self) -> Option<&'_ str> {
        (*self).topic()
    }

    fn data(&self) -> &'_ [u8] {
        (*self).data()
    }

    fn details(&self) -> &Details {
        (*self).details()
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageImpl {
    id: MessageId,
    topic: Option<alloc::string::String>,
    details: Details,
    data: alloc::vec::Vec<u8>,
}

#[cfg(feature = "alloc")]
impl MessageImpl {
    pub fn new<M>(message: &M) -> Self
    where
        M: Message,
    {
        Self {
            id: message.id(),
            data: message.data().to_vec(),
            topic: message.topic().map(alloc::string::String::from),
            details: message.details().clone(),
        }
    }
}

#[cfg(feature = "alloc")]
impl Message for MessageImpl {
    fn id(&self) -> MessageId {
        self.id
    }

    fn topic(&self) -> Option<&'_ str> {
        self.topic.as_deref()
    }

    fn data(&self) -> &'_ [u8] {
        &self.data
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

pub trait Client: ErrorType {
    fn subscribe<'a>(&'a mut self, topic: &'a str, qos: QoS) -> Result<MessageId, Self::Error>;

    fn unsubscribe<'a>(&'a mut self, topic: &'a str) -> Result<MessageId, Self::Error>;
}

impl<C> Client for &mut C
where
    C: Client,
{
    fn subscribe<'a>(&'a mut self, topic: &'a str, qos: QoS) -> Result<MessageId, Self::Error> {
        (*self).subscribe(topic, qos)
    }

    fn unsubscribe<'a>(&'a mut self, topic: &'a str) -> Result<MessageId, Self::Error> {
        (*self).unsubscribe(topic)
    }
}

pub trait Publish: ErrorType {
    fn publish<'a>(
        &'a mut self,
        topic: &'a str,
        qos: QoS,
        retain: bool,
        payload: &'a [u8],
    ) -> Result<MessageId, Self::Error>;
}

impl<P> Publish for &mut P
where
    P: Publish,
{
    fn publish<'a>(
        &'a mut self,
        topic: &'a str,
        qos: QoS,
        retain: bool,
        payload: &'a [u8],
    ) -> Result<MessageId, Self::Error> {
        (*self).publish(topic, qos, retain, payload)
    }
}

pub trait Enqueue: ErrorType {
    fn enqueue<'a>(
        &'a mut self,
        topic: &'a str,
        qos: QoS,
        retain: bool,
        payload: &'a [u8],
    ) -> Result<MessageId, Self::Error>;
}

impl<E> Enqueue for &mut E
where
    E: Enqueue,
{
    fn enqueue<'a>(
        &'a mut self,
        topic: &'a str,
        qos: QoS,
        retain: bool,
        payload: &'a [u8],
    ) -> Result<MessageId, Self::Error> {
        (*self).enqueue(topic, qos, retain, payload)
    }
}

pub trait Connection: ErrorType {
    type Message;

    fn next(&mut self) -> Option<Result<Event<Self::Message>, Self::Error>>;
}

impl<C> Connection for &mut C
where
    C: Connection,
{
    type Message = C::Message;

    fn next(&mut self) -> Option<Result<Event<Self::Message>, Self::Error>> {
        (*self).next()
    }
}

#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
pub mod utils {
    use core::fmt::Debug;
    use core::mem;

    use alloc::sync::Arc;

    use crate::{
        mutex::RawCondvar,
        utils::mutex::{Condvar, Mutex},
    };

    use super::{ErrorType, Event};

    pub struct ConnStateGuard<CV, S>
    where
        CV: RawCondvar,
    {
        pub state: Mutex<CV::RawMutex, Option<S>>,
        pub state_changed: Condvar<CV>,
    }

    impl<CV, S> ConnStateGuard<CV, S>
    where
        CV: RawCondvar,
    {
        pub fn new(state: S) -> Self {
            Self {
                state: Mutex::new(Some(state)),
                state_changed: Condvar::new(),
            }
        }
    }

    impl<CV, S> ConnStateGuard<CV, S>
    where
        CV: RawCondvar,
        S: Default,
    {
        pub fn new_default() -> Self {
            Self::new(Default::default())
        }
    }

    impl<CV, S> ConnStateGuard<CV, S>
    where
        CV: RawCondvar,
    {
        pub fn close(&self) {
            let mut state = self.state.lock();

            *state = None;
            self.state_changed.notify_all();
        }
    }

    impl<CV, S> Default for ConnStateGuard<CV, S>
    where
        CV: RawCondvar,
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
        CV: RawCondvar;

    impl<CV, M, E> Postbox<CV, M, E>
    where
        CV: RawCondvar,
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
        CV: RawCondvar;

    impl<CV, M, E> Connection<CV, M, E>
    where
        CV: RawCondvar,
        E: Debug,
    {
        pub fn new(connection_state: Arc<ConnStateGuard<CV, ConnState<M, E>>>) -> Self {
            Self(connection_state)
        }
    }

    impl<CV, M, E> ErrorType for Connection<CV, M, E>
    where
        CV: RawCondvar,
        E: Debug,
    {
        type Error = E;
    }

    impl<CV, M, E> super::Connection for Connection<CV, M, E>
    where
        CV: RawCondvar,
        E: Debug,
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

#[cfg(all(feature = "nightly", feature = "experimental"))]
pub mod asynch {
    use core::future::Future;

    use crate::executor::asynch::{Blocker, Blocking};

    pub use super::{Details, ErrorType, Event, Message, MessageId, QoS};

    pub trait Client: ErrorType {
        type SubscribeFuture<'a>: Future<Output = Result<MessageId, Self::Error>> + Send
        where
            Self: 'a;

        type UnsubscribeFuture<'a>: Future<Output = Result<MessageId, Self::Error>> + Send
        where
            Self: 'a;

        fn subscribe<'a>(&'a mut self, topic: &'a str, qos: QoS) -> Self::SubscribeFuture<'a>;

        fn unsubscribe<'a>(&'a mut self, topic: &'a str) -> Self::UnsubscribeFuture<'a>;
    }

    impl<C> Client for &mut C
    where
        C: Client,
    {
        type SubscribeFuture<'a>
        = C::SubscribeFuture<'a> where Self: 'a;

        type UnsubscribeFuture<'a>
        = C::UnsubscribeFuture<'a> where Self: 'a;

        fn subscribe<'a>(&'a mut self, topic: &'a str, qos: QoS) -> Self::SubscribeFuture<'a> {
            (*self).subscribe(topic, qos)
        }

        fn unsubscribe<'a>(&'a mut self, topic: &'a str) -> Self::UnsubscribeFuture<'a> {
            (*self).unsubscribe(topic)
        }
    }

    pub trait Publish: ErrorType {
        type PublishFuture<'a>: Future<Output = Result<MessageId, Self::Error>> + Send
        where
            Self: 'a;

        fn publish<'a>(
            &'a mut self,
            topic: &'a str,
            qos: QoS,
            retain: bool,
            payload: &'a [u8],
        ) -> Self::PublishFuture<'a>;
    }

    impl<P> Publish for &mut P
    where
        P: Publish,
    {
        type PublishFuture<'a>
        = P::PublishFuture<'a> where Self: 'a;

        fn publish<'a>(
            &'a mut self,
            topic: &'a str,
            qos: QoS,
            retain: bool,
            payload: &'a [u8],
        ) -> Self::PublishFuture<'a> {
            (*self).publish(topic, qos, retain, payload)
        }
    }

    /// core.stream.Stream is not stable yet and on top of that it has an Item which is not
    /// parameterizable by lifetime (GATs). Therefore, we have to use a Future instead
    pub trait Connection: ErrorType {
        type Message;

        type NextFuture<'a>: Future<Output = Option<Result<Event<Self::Message>, Self::Error>>>
            + Send
        where
            Self: 'a;

        fn next(&mut self) -> Self::NextFuture<'_>;
    }

    impl<C> Connection for &mut C
    where
        C: Connection,
    {
        type Message = C::Message;

        type NextFuture<'a>
        = C::NextFuture<'a> where Self: 'a;

        fn next(&mut self) -> Self::NextFuture<'_> {
            (*self).next()
        }
    }

    impl<B, C> super::ErrorType for Blocking<B, C>
    where
        C: ErrorType,
    {
        type Error = C::Error;
    }

    impl<B, C> super::Client for Blocking<B, C>
    where
        B: Blocker,
        C: Client,
    {
        fn subscribe<'a>(&'a mut self, topic: &'a str, qos: QoS) -> Result<MessageId, Self::Error> {
            self.blocker.block_on(self.api.subscribe(topic, qos))
        }

        fn unsubscribe<'a>(&'a mut self, topic: &'a str) -> Result<MessageId, Self::Error> {
            self.blocker.block_on(self.api.unsubscribe(topic))
        }
    }

    impl<B, P> super::Publish for Blocking<B, P>
    where
        B: Blocker,
        P: Publish,
    {
        fn publish<'a>(
            &'a mut self,
            topic: &'a str,
            qos: QoS,
            retain: bool,
            payload: &'a [u8],
        ) -> Result<MessageId, Self::Error> {
            self.blocker
                .block_on(self.api.publish(topic, qos, retain, payload))
        }
    }

    impl<B, C> super::Connection for Blocking<B, C>
    where
        B: Blocker,
        C: Connection,
    {
        type Message = C::Message;

        fn next(&mut self) -> Option<Result<Event<Self::Message>, Self::Error>> {
            self.blocker.block_on(self.api.next())
        }
    }
}
