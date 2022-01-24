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

pub trait Message {
    fn id(&self) -> MessageId;

    fn topic(&self, topic_token: &TopicToken) -> Cow<'_, str>;

    fn data(&self) -> Cow<'_, [u8]>;

    fn details(&self) -> &Details;
}

pub enum Details {
    Complete(TopicToken),
    InitialChunk(InitialChunkData),
    SubsequentChunk(SubsequentChunkData),
}

pub struct InitialChunkData {
    pub topic_token: TopicToken,
    pub total_data_size: usize,
}

pub struct SubsequentChunkData {
    pub current_data_offset: usize,
    pub total_data_size: usize,
}

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

    pub trait Connection: Service {
        type Message: Message;

        type Reference<'a>: Deref<Target = Option<Result<Event<Self::Message>, Self::Error>>>
        where
            Self: 'a;

        type NextFuture<'a>: Future<Output = Self::Reference<'a>>
        where
            Self: 'a;

        /// core.stream.Stream has an Item which is not parameterizable by lifetime (GATs)
        /// Therefore, we have to use a Future instead
        fn next(&mut self) -> Self::NextFuture<'_>;
    }
}
