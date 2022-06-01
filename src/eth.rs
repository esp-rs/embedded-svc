use core::convert::TryFrom;
use core::fmt::Debug;

use enumset::*;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "use_strum")]
use strum_macros::{Display, EnumIter, EnumMessage, EnumString};

#[cfg(feature = "use_numenum")]
use num_enum::TryFromPrimitive;

use crate::errors::{EitherError, Errors};
use crate::ipv4;

#[derive(EnumSetType, Debug, PartialOrd)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, Display, EnumMessage, EnumIter)
)]
#[cfg_attr(feature = "use_numenum", derive(TryFromPrimitive))]
#[cfg_attr(feature = "use_numenum", repr(u8))]
pub enum Capability {
    Client,
    Router,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum Configuration<S> {
    None,
    NOIP,
    Client(ipv4::ClientConfiguration<S>),
    Router(ipv4::RouterConfiguration),
}

pub trait TransitionalState<T> {
    fn is_transitional(&self) -> bool;
    fn is_operating(&self) -> bool;
    fn get_operating(&self) -> Option<&T>;
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum IpStatus {
    Disabled,
    Waiting,
    Done(Option<ipv4::ClientSettings>),
}

impl TransitionalState<Option<ipv4::ClientSettings>> for IpStatus {
    fn is_transitional(&self) -> bool {
        *self == IpStatus::Waiting
    }

    fn is_operating(&self) -> bool {
        *self != IpStatus::Disabled
    }

    fn get_operating(&self) -> Option<&Option<ipv4::ClientSettings>> {
        if let IpStatus::Done(ref settings) = *self {
            Some(settings)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected(IpStatus),
}

impl TransitionalState<IpStatus> for ConnectionStatus {
    fn is_transitional(&self) -> bool {
        *self == ConnectionStatus::Connecting
            || (if let ConnectionStatus::Connected(ips) = self {
                ips.is_transitional()
            } else {
                false
            })
    }

    fn is_operating(&self) -> bool {
        *self != ConnectionStatus::Disconnected
    }

    fn get_operating(&self) -> Option<&IpStatus> {
        if let ConnectionStatus::Connected(ref settings) = *self {
            Some(settings)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum Status {
    Stopped,
    Starting,
    Started(ConnectionStatus),
}

impl TransitionalState<ConnectionStatus> for Status {
    fn is_transitional(&self) -> bool {
        *self == Status::Starting
            || (if let Status::Started(ccs) = self {
                ccs.is_transitional()
            } else {
                false
            })
    }

    fn is_operating(&self) -> bool {
        *self != Status::Stopped
    }

    fn get_operating(&self) -> Option<&ConnectionStatus> {
        if let Status::Started(ref settings) = *self {
            Some(settings)
        } else {
            None
        }
    }
}

pub trait Eth: Errors {
    fn get_capabilities(&self) -> Result<EnumSet<Capability>, Self::Error>;

    fn get_status(&self) -> Status;

    fn get_configuration<S>(
        &self,
    ) -> Result<Configuration<S>, EitherError<Self::Error, <S as TryFrom<&str>>::Error>>
    where
        S: for<'a> TryFrom<&'a str> + 'static;
    fn set_configuration(&mut self, conf: &Configuration<&'_ str>) -> Result<(), Self::Error>;
}
