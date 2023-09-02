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
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

pub type MessageId = u32;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
            Self::Connected(connected) => write!(f, "Connected(session: {connected})"),
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Subscribed(message_id) => write!(f, "Subscribed({message_id})"),
            Self::Unsubscribed(message_id) => write!(f, "Unsubscribed({message_id})"),
            Self::Published(message_id) => write!(f, "Published({message_id})"),
            Self::Received(message) => write!(f, "Received({message})"),
            Self::Deleted(message_id) => write!(f, "Deleted({message_id})"),
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
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Details {
    Complete,
    InitialChunk(InitialChunkData),
    SubsequentChunk(SubsequentChunkData),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct InitialChunkData {
    pub total_data_size: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
    type Message<'a>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Result<Event<Self::Message<'_>>, Self::Error>>;
}

impl<C> Connection for &mut C
where
    C: Connection,
{
    type Message<'a> = C::Message<'a> where Self: 'a;

    fn next(&mut self) -> Option<Result<Event<Self::Message<'_>>, Self::Error>> {
        (*self).next()
    }
}

#[cfg(feature = "nightly")]
pub mod asynch {
    pub use super::{Details, ErrorType, Event, Message, MessageId, QoS};

    pub trait Client: ErrorType {
        async fn subscribe(&mut self, topic: &str, qos: QoS) -> Result<MessageId, Self::Error>;

        async fn unsubscribe(&mut self, topic: &str) -> Result<MessageId, Self::Error>;
    }

    impl<C> Client for &mut C
    where
        C: Client,
    {
        async fn subscribe(&mut self, topic: &str, qos: QoS) -> Result<MessageId, Self::Error> {
            (*self).subscribe(topic, qos).await
        }

        async fn unsubscribe(&mut self, topic: &str) -> Result<MessageId, Self::Error> {
            (*self).unsubscribe(topic).await
        }
    }

    pub trait Publish: ErrorType {
        async fn publish(
            &mut self,
            topic: &str,
            qos: QoS,
            retain: bool,
            payload: &[u8],
        ) -> Result<MessageId, Self::Error>;
    }

    impl<P> Publish for &mut P
    where
        P: Publish,
    {
        async fn publish(
            &mut self,
            topic: &str,
            qos: QoS,
            retain: bool,
            payload: &[u8],
        ) -> Result<MessageId, Self::Error> {
            (*self).publish(topic, qos, retain, payload).await
        }
    }

    /// core.stream.Stream is not stable yet and on top of that it has an Item which is not
    /// parameterizable by lifetime (GATs). Therefore, we have to use a Future instead
    pub trait Connection: ErrorType {
        type Message<'a>
        where
            Self: 'a;

        async fn next(&mut self) -> Option<Result<Event<Self::Message<'_>>, Self::Error>>;
    }

    impl<C> Connection for &mut C
    where
        C: Connection,
    {
        type Message<'a> = C::Message<'a> where Self: 'a;

        async fn next(&mut self) -> Option<Result<Event<Self::Message<'_>>, Self::Error>> {
            (*self).next().await
        }
    }
}
