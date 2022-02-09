use core::fmt::{Debug, Display};
use core::marker::PhantomData;

extern crate alloc;
use alloc::borrow::Cow;

use crate::service::Service;

/// Quality of service
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

pub type MessageId = u32;

pub enum Event<M>
where
    M: Message,
{
    BeforeConnect,
    Connected(bool),
    Disconnected,
    Subscribed(MessageId),
    Unsubscribed(MessageId),
    Published(MessageId),
    Received(M),
    Deleted(MessageId),
}

impl<M: Message> Display for Event<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BeforeConnect => write!(f, "BeforeConnect"),
            Self::Connected(connected) => write!(f, "Connected(session: {})", connected),
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Subscribed(message_id) => write!(f, "Subscribed({})", message_id),
            Self::Unsubscribed(message_id) => write!(f, "Unsubscribed({})", message_id),
            Self::Published(message_id) => write!(f, "Published({})", message_id),
            Self::Received(message) => write!(f, "Received({})", message.id()),
            Self::Deleted(message_id) => write!(f, "Deleted({})", message_id),
        }
    }
}

impl<M: Message> Debug for Event<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Received(message) => {
                let topic_token = match message.details() {
                    Details::Complete(topic_token) => Some(topic_token),
                    Details::InitialChunk(data) => Some(&data.topic_token),
                    _ => None,
                };

                let topic = topic_token.map(|topic_token| message.topic(topic_token));

                write!(
                    f,
                    "
[
    id: {},
    topic: {:?},
    data: {:?},
    details: {:?}
]",
                    message.id(),
                    topic,
                    message.data(),
                    message.details()
                )
            }
            other => write!(f, "{}", other),
        }
    }
}

pub trait Message {
    fn id(&self) -> MessageId;

    fn topic(&self, topic_token: &TopicToken) -> Cow<'_, str>;

    fn data(&self) -> Cow<'_, [u8]>;

    fn details(&self) -> &Details;
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

pub trait Client: Service {
    fn subscribe<'a, S>(&'a mut self, topic: S, qos: QoS) -> Result<MessageId, Self::Error>
    where
        S: Into<Cow<'a, str>>;

    fn unsubscribe<'a, S>(&'a mut self, topic: S) -> Result<MessageId, Self::Error>
    where
        S: Into<Cow<'a, str>>;
}

pub trait Publish: Service {
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

pub trait Enqueue: Service {
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

pub trait Connection: Service {
    type Message<'a>: Message
    where
        Self: 'a;

    /// GATs do not (yet) define a standard streaming iterator,
    /// so we have to put the next() method directly in the Connection trait
    fn next(&mut self) -> Option<Result<Event<Self::Message<'_>>, Self::Error>>;
}

pub mod nonblocking {
    use core::future::Future;
    use core::ops::Deref;

    extern crate alloc;
    use alloc::borrow::Cow;

    pub use super::{Client, Event, Message, MessageId, QoS};

    use crate::service::Service;

    pub trait Publish: Service {
        type PublishFuture: Future<Output = Result<MessageId, Self::Error>>;

        fn publish<'a, S, V>(
            &'a mut self,
            topic: S,
            qos: QoS,
            retain: bool,
            payload: V,
        ) -> Self::PublishFuture
        where
            S: Into<Cow<'a, str>>,
            V: Into<Cow<'a, [u8]>>;
    }

    /// core.stream.Stream is not stable yet and on top of that it has an Item which is not
    /// parameterizable by lifetime (GATs). Therefore, we have to use a Future instead
    pub trait Connection: Service {
        type Message<'a>: Message
        where
            Self: 'a;

        type Reference<'a>: Deref<Target = Option<Result<Event<Self::Message<'a>>, Self::Error>>>
        where
            Self: 'a;

        type NextFuture<'a>: Future<Output = Self::Reference<'a>>
        where
            Self: 'a;

        fn next(&mut self) -> Self::NextFuture<'_>;
    }
}
