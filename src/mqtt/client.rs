use core::fmt::{self, Debug, Display, Formatter};

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "use_serde")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

pub type MessageId = u32;

pub trait Event: ErrorType {
    fn payload(&self) -> EventPayload<'_, Self::Error>;
}

impl<E> Event for &E
where
    E: Event,
{
    fn payload(&self) -> EventPayload<'_, Self::Error> {
        (*self).payload()
    }
}

impl<E> Event for &mut E
where
    E: Event,
{
    fn payload(&self) -> EventPayload<'_, Self::Error> {
        (**self).payload()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum EventPayload<'a, E> {
    BeforeConnect,
    Connected(bool),
    Disconnected,
    Subscribed(MessageId),
    Unsubscribed(MessageId),
    Published(MessageId),
    Received {
        id: MessageId,
        topic: Option<&'a str>,
        data: &'a [u8],
        details: Details,
    },
    Deleted(MessageId),
    Error(&'a E),
}

impl<'a, E> Display for EventPayload<'a, E>
where
    E: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::BeforeConnect => write!(f, "BeforeConnect"),
            Self::Connected(session_present) => write!(f, "Connected(session: {session_present})"),
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Subscribed(message_id) => write!(f, "Subscribed({message_id})"),
            Self::Unsubscribed(message_id) => write!(f, "Unsubscribed({message_id})"),
            Self::Published(message_id) => write!(f, "Published({message_id})"),
            Self::Received {
                id,
                topic,
                data,
                details,
            } => write!(
                f,
                "Received {{ id: {id}, topic: {topic:?}, data: {:?}, details: {details:?} }}",
                core::str::from_utf8(data),
            ),
            Self::Deleted(message_id) => write!(f, "Deleted({message_id})"),
            Self::Error(error) => write!(f, "Error({error:?})"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum Details {
    Complete,
    InitialChunk(InitialChunkData),
    SubsequentChunk(SubsequentChunkData),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct InitialChunkData {
    pub total_data_size: usize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
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
    type Event<'a>: Event
    where
        Self: 'a;

    fn next(&mut self) -> Result<Self::Event<'_>, Self::Error>;
}

impl<C> Connection for &mut C
where
    C: Connection,
{
    type Event<'a> = C::Event<'a> where Self: 'a;

    fn next(&mut self) -> Result<Self::Event<'_>, Self::Error> {
        (*self).next()
    }
}

pub mod asynch {
    pub use super::{Details, ErrorType, Event, EventPayload, MessageId, QoS};

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

    pub trait Connection: ErrorType {
        type Event<'a>: Event
        where
            Self: 'a;

        async fn next(&mut self) -> Result<Self::Event<'_>, Self::Error>;
    }

    impl<C> Connection for &mut C
    where
        C: Connection,
    {
        type Event<'a> = C::Event<'a> where Self: 'a;

        async fn next(&mut self) -> Result<Self::Event<'_>, Self::Error> {
            (*self).next().await
        }
    }
}
