use core::marker::PhantomData;

extern crate alloc;
use alloc::borrow::Cow;

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
    pub unsafe fn new() -> Self {
        Self(PhantomData)
    }
}

pub trait Client: Send {
    type Error;

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

    fn subscribe<'a, S>(&'a mut self, topic: S, qos: QoS) -> Result<MessageId, Self::Error>
    where
        S: Into<Cow<'a, str>>;

    fn unsubscribe<'a, S>(&'a mut self, topic: S) -> Result<MessageId, Self::Error>
    where
        S: Into<Cow<'a, str>>;
}

pub trait Connection: Send {
    type Error;

    type Message<'a>: Message;

    /// GATs do not (yet) define a standard streaming iterator,
    /// so we have to put the next() method directly in the Connection trait
    fn next(&mut self) -> Option<Result<Event<Self::Message<'_>>, Self::Error>>;
}
