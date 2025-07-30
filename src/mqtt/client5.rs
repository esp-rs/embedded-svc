use core::fmt::Debug;

#[cfg(feature = "std")]
use std::vec::Vec;

use crate::mqtt::client::{ErrorType, MessageId, QoS};

#[allow(unused_imports)]
pub use super::*;

extern crate alloc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserPropertyItem<'a> {
    pub key: &'a str,
    pub value: &'a str,
}

impl<'a> UserPropertyItem<'a> {
    pub fn new(key: &'a str, value: &'a str) -> Self {
        Self { key, value }
    }
}

/// MQTT5 protocol error reason codes as defined in MQTT5 protocol document section 2.4
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum ErrorReasonCode {
    /// Unspecified error
    UnspecifiedError = 0x80,
    /// The received packet does not conform to this specification
    MalformedPacket = 0x81,
    /// An unexpected or out of order packet was received
    ProtocolError = 0x82,
    /// Implementation specific error
    ImplementSpecificError = 0x83,
    /// The server does not support the level of the MQTT protocol requested by the client
    UnsupportedProtocolVersion = 0x84,
    /// The client identifier is not valid
    InvalidClientId = 0x85,
    /// The server does not accept the user name or password specified by the client
    BadUsernameOrPassword = 0x86,
    /// The client is not authorized to connect
    NotAuthorized = 0x87,
    /// The MQTT server is not available
    ServerUnavailable = 0x88,
    /// The server is busy. Try again later
    ServerBusy = 0x89,
    /// This client has been banned by administrative action
    Banned = 0x8A,
    /// The server is shutting down
    ServerShuttingDown = 0x8B,
    /// The authentication method is not supported
    BadAuthMethod = 0x8C,
    /// The connection is closed because no packet has been received for 1.5 times the keep alive time
    KeepAliveTimeout = 0x8D,
    /// Another connection using the same client ID has connected
    SessionTakenOver = 0x8E,
    /// The topic filter is not valid
    TopicFilterInvalid = 0x8F,
    /// The topic name is not valid
    TopicNameInvalid = 0x90,
    /// The packet identifier is already in use
    PacketIdentifierInUse = 0x91,
    /// The packet identifier is not found
    PacketIdentifierNotFound = 0x92,
    /// The client has received more than receive maximum publication
    ReceiveMaximumExceeded = 0x93,
    /// The topic alias is not valid
    TopicAliasInvalid = 0x94,
    /// The packet exceeded the maximum permissible size
    PacketTooLarge = 0x95,
    /// The message rate is too high
    MessageRateTooHigh = 0x96,
    /// An implementation or administrative imposed limit has been exceeded
    QuotaExceeded = 0x97,
    /// The connection is closed due to an administrative action
    AdministrativeAction = 0x98,
    /// The payload format does not match the specified format indicator
    PayloadFormatInvalid = 0x99,
    /// The server does not support retained messages
    RetainNotSupported = 0x9A,
    /// The server does not support the QoS requested
    QosNotSupported = 0x9B,
    /// The client should temporarily use another server
    UseAnotherServer = 0x9C,
    /// The server has moved and the client should permanently use another server
    ServerMoved = 0x9D,
    /// The server does not support shared subscriptions
    SharedSubscriptionNotSupported = 0x9E,
    /// The connection rate limit has been exceeded
    ConnectionRateExceeded = 0x9F,
    /// The maximum connection time authorized has been exceeded
    MaximumConnectTime = 0xA0,
    /// The server does not support subscription identifiers
    SubscribeIdentifierNotSupported = 0xA1,
    /// The server does not support wildcard subscriptions
    WildcardSubscriptionNotSupported = 0xA2,
}

impl core::fmt::Display for ErrorReasonCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ErrorReasonCode::UnspecifiedError => write!(f, "Unspecified error"),
            ErrorReasonCode::MalformedPacket => write!(f, "Malformed packet"),
            ErrorReasonCode::ProtocolError => write!(f, "Protocol error"),
            ErrorReasonCode::ImplementSpecificError => write!(f, "Implementation specific error"),
            ErrorReasonCode::UnsupportedProtocolVersion => {
                write!(f, "Unsupported protocol version")
            }
            ErrorReasonCode::InvalidClientId => write!(f, "Invalid client ID"),
            ErrorReasonCode::BadUsernameOrPassword => write!(f, "Bad username or password"),
            ErrorReasonCode::NotAuthorized => write!(f, "Not authorized"),
            ErrorReasonCode::ServerUnavailable => write!(f, "Server unavailable"),
            ErrorReasonCode::ServerBusy => write!(f, "Server busy"),
            ErrorReasonCode::Banned => write!(f, "Banned"),
            ErrorReasonCode::ServerShuttingDown => write!(f, "Server shutting down"),
            ErrorReasonCode::BadAuthMethod => write!(f, "Bad authentication method"),
            ErrorReasonCode::KeepAliveTimeout => write!(f, "Keep alive timeout"),
            ErrorReasonCode::SessionTakenOver => write!(f, "Session taken over"),
            ErrorReasonCode::TopicFilterInvalid => write!(f, "Topic filter invalid"),
            ErrorReasonCode::TopicNameInvalid => write!(f, "Topic name invalid"),
            ErrorReasonCode::PacketIdentifierInUse => write!(f, "Packet identifier in use"),
            ErrorReasonCode::PacketIdentifierNotFound => write!(f, "Packet identifier not found"),
            ErrorReasonCode::ReceiveMaximumExceeded => write!(f, "Receive maximum exceeded"),
            ErrorReasonCode::TopicAliasInvalid => write!(f, "Topic alias invalid"),
            ErrorReasonCode::PacketTooLarge => write!(f, "Packet too large"),
            ErrorReasonCode::MessageRateTooHigh => write!(f, "Message rate too high"),
            ErrorReasonCode::QuotaExceeded => write!(f, "Quota exceeded"),
            ErrorReasonCode::AdministrativeAction => write!(f, "Administrative action"),
            ErrorReasonCode::PayloadFormatInvalid => write!(f, "Payload format invalid"),
            ErrorReasonCode::RetainNotSupported => write!(f, "Retain not supported"),
            ErrorReasonCode::QosNotSupported => write!(f, "QoS not supported"),
            ErrorReasonCode::UseAnotherServer => write!(f, "Use another server"),
            ErrorReasonCode::ServerMoved => write!(f, "Server moved"),
            ErrorReasonCode::SharedSubscriptionNotSupported => {
                write!(f, "Shared subscription not supported")
            }
            ErrorReasonCode::ConnectionRateExceeded => write!(f, "Connection rate exceeded"),
            ErrorReasonCode::MaximumConnectTime => write!(f, "Maximum connect time"),
            ErrorReasonCode::SubscribeIdentifierNotSupported => {
                write!(f, "Subscribe identifier not supported")
            }
            ErrorReasonCode::WildcardSubscriptionNotSupported => {
                write!(f, "Wildcard subscription not supported")
            }
        }
    }
}

impl ErrorReasonCode {
    /// Returns the numeric code value for this error reason
    pub fn code(&self) -> u32 {
        *self as u32
    }

    /// Returns true if this is a client-side error (codes 0x80-0x8F)
    pub fn is_client_error(&self) -> bool {
        (*self as u32) <= 0x8F
    }

    /// Returns true if this is a server-side error (codes 0x90+)
    pub fn is_server_error(&self) -> bool {
        (*self as u32) >= 0x90
    }

    /// Returns true if this error indicates the connection should be retried
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ErrorReasonCode::ServerUnavailable
                | ErrorReasonCode::ServerBusy
                | ErrorReasonCode::UseAnotherServer
                | ErrorReasonCode::ConnectionRateExceeded
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageMetadata<'a> {
    pub payload_format_indicator: bool,
    pub response_topic: Option<&'a str>,
    pub correlation_data: Option<&'a [u8]>,
    pub content_type: Option<&'a str>,
    pub subscribe_id: u16,
}

impl<'a> MessageMetadata<'a> {
    pub fn new(
        payload_format_indicator: bool,
        response_topic: Option<&'a str>,
        correlation_data: Option<&'a [u8]>,
        content_type: Option<&'a str>,
        subscribe_id: u16,
    ) -> Self {
        MessageMetadata {
            payload_format_indicator,
            response_topic,
            correlation_data,
            content_type,
            subscribe_id,
        }
    }
}

pub trait UserPropertyList<TError> {
    fn set_items(&mut self, properties: &[UserPropertyItem]) -> Result<(), TError>;
    #[cfg(feature = "std")]
    fn get_items(&self) -> Result<Option<Vec<UserPropertyItem>>, TError>;
    fn clear(&self);
    fn count(&self) -> u8;
    fn is_empty(&self) -> bool {
        self.count() == 0
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PublishPropertyConfig<'a> {
    pub payload_format_indicator: bool,
    pub message_expiry_interval: u32,
    pub topic_alias: u16,
    pub response_topic: Option<&'a str>,
    pub correlation_data: Option<&'a [u8]>,
    pub content_type: Option<&'a str>,
    pub user_properties: Option<&'a [UserPropertyItem<'a>]>,
}

impl<'a> Default for PublishPropertyConfig<'a> {
    fn default() -> Self {
        Self {
            payload_format_indicator: false,
            message_expiry_interval: 0,
            topic_alias: 0,
            response_topic: None,
            correlation_data: None,
            content_type: None,
            user_properties: None,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SubscribePropertyConfig<'a> {
    pub subscribe_id: u16,
    pub no_local: bool,
    pub retain_as_published: bool,
    pub retain_handling: u8,
    pub share_name: Option<&'a str>,
    pub user_properties: Option<&'a [UserPropertyItem<'a>]>,
}

impl<'a> Default for SubscribePropertyConfig<'a> {
    fn default() -> Self {
        Self {
            subscribe_id: 0,
            no_local: false,
            retain_as_published: false,
            retain_handling: 0, // Default to 0 (Send retained messages)
            share_name: None,
            user_properties: None,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct UnsubscribePropertyConfig<'a> {
    pub is_shared: bool,
    pub share_name: Option<&'a str>,
    pub user_properties: Option<&'a [UserPropertyItem<'a>]>,
}

impl<'a> Default for UnsubscribePropertyConfig<'a> {
    fn default() -> Self {
        Self {
            is_shared: false,
            share_name: None,
            user_properties: None,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DisconnectPropertyConfig<'a> {
    pub session_expiry_interval: u32,
    pub reason: u8,
    pub user_properties: Option<&'a [UserPropertyItem<'a>]>,
}

impl<'a> Default for DisconnectPropertyConfig<'a> {
    fn default() -> Self {
        Self {
            session_expiry_interval: 0,
            reason: 0,
            user_properties: None,
        }
    }
}

pub trait Client: ErrorType {
    fn subscribe<'a>(
        &mut self,
        topic: &str,
        qos: QoS,
        config: Option<SubscribePropertyConfig<'a>>,
    ) -> Result<MessageId, Self::Error>;

    fn unsubscribe<'a>(
        &mut self,
        topic: &str,
        config: Option<UnsubscribePropertyConfig<'a>>,
    ) -> Result<MessageId, Self::Error>;

    fn disconnect<'a>(
        &mut self,
        config: Option<DisconnectPropertyConfig<'a>>,
    ) -> Result<(), Self::Error>;
}

impl<C> Client for &mut C
where
    C: Client,
{
    fn subscribe<'a>(
        &mut self,
        topic: &str,
        qos: QoS,
        config: Option<SubscribePropertyConfig<'a>>,
    ) -> Result<MessageId, Self::Error> {
        (*self).subscribe(topic, qos, config)
    }

    fn unsubscribe<'a>(
        &mut self,
        topic: &str,
        config: Option<UnsubscribePropertyConfig<'a>>,
    ) -> Result<MessageId, Self::Error> {
        (*self).unsubscribe(topic, config)
    }

    fn disconnect<'a>(
        &mut self,
        config: Option<DisconnectPropertyConfig<'a>>,
    ) -> Result<(), Self::Error> {
        (*self).disconnect(config)
    }
}

pub trait Publish: ErrorType {
    fn publish<'a>(
        &mut self,
        topic: &str,
        qos: QoS,
        retain: bool,
        payload: &'a [u8],
        config: Option<PublishPropertyConfig<'a>>,
    ) -> Result<MessageId, Self::Error>;
}

impl<P> Publish for &mut P
where
    P: Publish,
{
    fn publish<'a>(
        &mut self,
        topic: &str,
        qos: QoS,
        retain: bool,
        payload: &'a [u8],
        config: Option<PublishPropertyConfig<'a>>,
    ) -> Result<MessageId, Self::Error> {
        (*self).publish(topic, qos, retain, payload, config)
    }
}

pub mod asyncch {
    use crate::mqtt::{
        client::{ErrorType, MessageId, QoS},
        client5::{
            DisconnectPropertyConfig, PublishPropertyConfig, SubscribePropertyConfig,
            UnsubscribePropertyConfig,
        },
    };

    pub trait Client: ErrorType {
        async fn subscribe<'a>(
            &'a mut self,
            topic: &'a str,
            qos: QoS,
            config: Option<SubscribePropertyConfig<'a>>,
        ) -> Result<MessageId, Self::Error>;

        async fn unsubscribe<'a>(
            &'a mut self,
            topic: &'a str,
            config: Option<UnsubscribePropertyConfig<'a>>,
        ) -> Result<MessageId, Self::Error>;

        async fn disconnect<'a>(
            &'a mut self,
            config: Option<DisconnectPropertyConfig<'a>>,
        ) -> Result<(), Self::Error>;
    }

    impl<C> Client for &mut C
    where
        C: Client,
    {
        async fn subscribe<'a>(
            &'a mut self,
            topic: &'a str,
            qos: QoS,
            config: Option<SubscribePropertyConfig<'a>>,
        ) -> Result<MessageId, Self::Error> {
            (*self).subscribe(topic, qos, config).await
        }

        async fn unsubscribe<'a>(
            &'a mut self,
            topic: &'a str,
            config: Option<UnsubscribePropertyConfig<'a>>,
        ) -> Result<MessageId, Self::Error> {
            (*self).unsubscribe(topic, config).await
        }

        async fn disconnect<'a>(
            &'a mut self,
            config: Option<DisconnectPropertyConfig<'a>>,
        ) -> Result<(), Self::Error> {
            (*self).disconnect(config).await
        }
    }

    pub trait Publish: ErrorType {
        async fn publish<'a>(
            &'a mut self,
            topic: &'a str,
            qos: QoS,
            retain: bool,
            payload: &'a [u8],
            config: Option<PublishPropertyConfig<'a>>,
        ) -> Result<MessageId, Self::Error>;
    }

    impl<P> Publish for &mut P
    where
        P: Publish,
    {
        async fn publish<'a>(
            &'a mut self,
            topic: &'a str,
            qos: QoS,
            retain: bool,
            payload: &'a [u8],
            config: Option<PublishPropertyConfig<'a>>,
        ) -> Result<MessageId, Self::Error> {
            (*self).publish(topic, qos, retain, payload, config).await
        }
    }
}
