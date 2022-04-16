use core::fmt::{self, Debug, Display, Formatter};
use core::marker::PhantomData;

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

    fn topic(&self, topic_token: &TopicToken) -> Cow<'_, str>;

    fn data(&self) -> Cow<'_, [u8]>;

    fn details(&self) -> &Details;

    fn retrieve_topic(&self) -> Option<Cow<'_, str>> {
        let topic_token = match self.details() {
            Details::Complete(topic_token) => Some(topic_token),
            Details::InitialChunk(chunk) => Some(&chunk.topic_token),
            _ => None,
        };

        topic_token.map(|topic_token| self.topic(topic_token))
    }
}

#[derive(Debug)]
pub enum Details {
    Complete(TopicToken),
    InitialChunk(InitialChunkData),
    SubsequentChunk(SubsequentChunkData),
}

#[derive(Debug)]
pub struct InitialChunkData {
    pub topic_token: TopicToken,
    pub total_data_size: usize,
}

#[derive(Debug)]
pub struct SubsequentChunkData {
    pub current_data_offset: usize,
    pub total_data_size: usize,
}

#[derive(Debug)]
pub struct TopicToken(PhantomData<*const ()>);

impl TopicToken {
    /// # Safety
    /// This function is marked as unsafe because it is an internal API and is NOT supposed to be called by the user
    pub unsafe fn new() -> Self {
        Self(PhantomData)
    }
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
    type Message<'a>: Message
    where
        Self: 'a;

    /// GATs do not (yet) define a standard streaming iterator,
    /// so we have to put the next() method directly in the Connection trait
    fn next(&mut self) -> Option<Result<Event<Self::Message<'_>>, Self::Error>>;
}

impl<'b, C> Connection for &'b mut C
where
    C: Connection,
{
    type Message<'a>
    where
        Self: 'a,
    = C::Message<'a>;

    fn next(&mut self) -> Option<Result<Event<Self::Message<'_>>, Self::Error>> {
        (*self).next()
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
        type Message: Message;

        type NextFuture<'a, F, O>: Future<Output = Option<O>> + Send
        where
            Self: 'a,
            F: FnOnce(&Result<Event<Self::Message>, Self::Error>) -> O + Unpin + Send;

        fn next<'a, F, O>(&'a mut self, f: F) -> Self::NextFuture<'a, F, O>
        where
            F: FnOnce(&Result<Event<Self::Message>, Self::Error>) -> O + Unpin + Send;
    }
}
