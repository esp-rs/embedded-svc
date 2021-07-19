use core::fmt::Debug;

use enumset::*;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "use_strum")]
use strum_macros::{EnumIter, EnumMessage, EnumString, ToString};

#[cfg(feature = "use_numenum")]
use num_enum::TryFromPrimitive;

use crate::ipv4;

#[derive(EnumSetType, Debug, PartialOrd)]
#[cfg_attr(feature = "std", derive(Hash))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, ToString, EnumMessage, EnumIter)
)]
#[cfg_attr(feature = "use_numenum", derive(TryFromPrimitive))]
#[cfg_attr(feature = "use_numenum", repr(u8))]
pub enum OperationMode {
    Client,
    Router,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum Configuration {
    None,
    Client(ipv4::ClientConfiguration),
    Router(ipv4::RouterConfiguration),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum ClientIpStatus {
    Disabled,
    Waiting,
    Done(ipv4::ClientSettings),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum ClientConnectionStatus {
    Disconnected,
    Connecting,
    Connected(ClientIpStatus),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum ClientStatus {
    Stopped,
    Starting,
    Started(ClientConnectionStatus),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum RouterIpStatus {
    Disabled,
    Waiting,
    Done,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum RouterStatus {
    Stopped,
    Starting,
    Started(RouterIpStatus),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct Status(pub ClientStatus, pub RouterStatus);

pub trait Eth {
    type Error;

    fn get_supported_operation_modes(&self) -> Result<EnumSet<OperationMode>, Self::Error>;

    fn get_status(&self) -> Status;

    fn get_configuration(&self) -> Result<Configuration, Self::Error>;
    fn set_configuration(&mut self, conf: &Configuration) -> Result<(), Self::Error>;
}

#[cfg(feature = "alloc")]
pub struct AnyhowEth<T>(pub T);

#[cfg(feature = "alloc")]
impl<E, H> Eth for AnyhowEth<H>
where
    E: Into<anyhow::Error>,
    H: Eth<Error = E>,
{
    type Error = anyhow::Error;

    fn get_supported_operation_modes(&self) -> Result<EnumSet<OperationMode>, Self::Error> {
        self.0.get_supported_operation_modes().map_err(Into::into)
    }

    fn get_status(&self) -> Status {
        self.0.get_status()
    }

    fn get_configuration(&self) -> Result<Configuration, Self::Error> {
        self.0.get_configuration().map_err(Into::into)
    }

    fn set_configuration(&mut self, conf: &Configuration) -> Result<(), Self::Error> {
        self.0.set_configuration(conf).map_err(Into::into)
    }
}
