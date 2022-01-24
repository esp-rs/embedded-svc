// use crate::mqtt::client::Enqueue;

// impl crate::mqtt::client::nonblocking::Publish for Enqueue {
//     type PublishFuture: Future<Output = Result<MessageId, Self::Error>>;

//     fn publish<'a, S, V>(
//         &'a mut self,
//         topic: S,
//         qos: QoS,
//         retain: bool,
//         payload: V,
//     ) -> Self::PublishFuture
//     where
//         S: Into<Cow<'a, str>>,
//         V: Into<Cow<'a, [u8]>>;
// }

// impl crate::mqtt::client::nonblocking::Connection for Connection {
//     type Message<'a>: Message
//     where
//         Self: 'a;

//     type NextFuture<'a>: Future<Output = Option<Result<Event<Self::Message<'a>>, Self::Error>>>
//     where
//         Self: 'a;

//     /// core.stream.Stream has an Item which is not parameterizable by lifetime (GATs)
//     /// Therefore, we have to use a Future instead
//     fn next(&mut self) -> Self::NextFuture<'_>;
// }
