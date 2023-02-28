use core::fmt::Debug;
use core::mem;

#[cfg(feature = "alloc")]
extern crate alloc;

use enumset::*;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "use_strum")]
use strum_macros::{Display, EnumIter, EnumMessage, EnumString, EnumVariantNames, FromRepr};

#[cfg(feature = "use_numenum")]
use num_enum::TryFromPrimitive;

#[derive(EnumSetType, Debug, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, Display, EnumMessage, EnumIter, EnumVariantNames, FromRepr)
)]
#[cfg_attr(feature = "use_numenum", derive(TryFromPrimitive))]
#[cfg_attr(feature = "use_numenum", repr(u8))]
pub enum AuthMethod {
    #[cfg_attr(feature = "use_strum", strum(serialize = "none", message = "None"))]
    None,
    #[cfg_attr(feature = "use_strum", strum(serialize = "wep", message = "WEP"))]
    WEP,
    #[cfg_attr(feature = "use_strum", strum(serialize = "wpa", message = "WPA"))]
    WPA,
    #[cfg_attr(
        feature = "use_strum",
        strum(serialize = "wpa2personal", message = "WPA2 Personal")
    )]
    WPA2Personal,
    #[cfg_attr(
        feature = "use_strum",
        strum(serialize = "wpawpa2personal", message = "WPA & WPA2 Personal")
    )]
    WPAWPA2Personal,
    #[cfg_attr(
        feature = "use_strum",
        strum(serialize = "wpa2enterprise", message = "WPA2 Enterprise")
    )]
    WPA2Enterprise,
    #[cfg_attr(
        feature = "use_strum",
        strum(serialize = "wpa3personal", message = "WPA3 Personal")
    )]
    WPA3Personal,
    #[cfg_attr(
        feature = "use_strum",
        strum(serialize = "wpa2wpa3personal", message = "WPA2 & WPA3 Personal")
    )]
    WPA2WPA3Personal,
    #[cfg_attr(
        feature = "use_strum",
        strum(serialize = "wapipersonal", message = "WAPI Personal")
    )]
    WAPIPersonal,
}

impl Default for AuthMethod {
    fn default() -> Self {
        AuthMethod::WPA2Personal
    }
}

#[derive(EnumSetType, Debug, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, Display, EnumMessage, EnumIter, EnumVariantNames, FromRepr)
)]
#[cfg_attr(feature = "use_numenum", derive(TryFromPrimitive))]
#[cfg_attr(feature = "use_numenum", repr(u8))]
pub enum Protocol {
    #[cfg_attr(
        feature = "use_strum",
        strum(serialize = "p802d11b", message = "802.11B")
    )]
    P802D11B,
    #[cfg_attr(
        feature = "use_strum",
        strum(serialize = "p802d11bg", message = "802.11BG")
    )]
    P802D11BG,
    #[cfg_attr(
        feature = "use_strum",
        strum(serialize = "p802d11bgn", message = "802.11BGN")
    )]
    P802D11BGN,
    #[cfg_attr(
        feature = "use_strum",
        strum(serialize = "p802d11bgnlr", message = "802.11BGNLR")
    )]
    P802D11BGNLR,
    #[cfg_attr(
        feature = "use_strum",
        strum(serialize = "p802d11lr", message = "802.11LR")
    )]
    P802D11LR,
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::P802D11BGN
    }
}

#[derive(EnumSetType, Debug, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, Display, EnumMessage, EnumIter, EnumVariantNames, FromRepr)
)]
#[cfg_attr(feature = "use_numenum", derive(TryFromPrimitive))]
#[cfg_attr(feature = "use_numenum", repr(u8))]
pub enum SecondaryChannel {
    // TODO: Need to extend that for 5GHz
    #[cfg_attr(feature = "use_strum", strum(serialize = "none", message = "None"))]
    None,
    #[cfg_attr(feature = "use_strum", strum(serialize = "above", message = "Above"))]
    Above,
    #[cfg_attr(feature = "use_strum", strum(serialize = "below", message = "Below"))]
    Below,
}

impl Default for SecondaryChannel {
    fn default() -> Self {
        SecondaryChannel::None
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct AccessPointInfo {
    pub ssid: heapless::String<32>,
    pub bssid: [u8; 6],
    pub channel: u8,
    pub secondary_channel: SecondaryChannel,
    pub signal_strength: i8,
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub protocols: EnumSet<Protocol>,
    pub auth_method: AuthMethod,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct AccessPointConfiguration {
    pub ssid: heapless::String<32>,
    pub ssid_hidden: bool,
    pub channel: u8,
    pub secondary_channel: Option<u8>,
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub protocols: EnumSet<Protocol>,
    pub auth_method: AuthMethod,
    pub password: heapless::String<64>,
    pub max_connections: u16,
}

impl Default for AccessPointConfiguration {
    fn default() -> Self {
        Self {
            ssid: "iot-device".into(),
            ssid_hidden: false,
            channel: 1,
            secondary_channel: None,
            protocols: Protocol::P802D11B | Protocol::P802D11BG | Protocol::P802D11BGN,
            auth_method: AuthMethod::None,
            password: "".into(),
            max_connections: 255,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct ClientConfiguration {
    pub ssid: heapless::String<32>,
    pub bssid: Option<[u8; 6]>,
    //pub protocol: Protocol,
    pub auth_method: AuthMethod,
    pub password: heapless::String<64>,
    pub channel: Option<u8>,
}

impl Debug for ClientConfiguration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ClientConfiguration")
            .field("ssid", &self.ssid)
            .field("bssid", &self.bssid)
            .field("auth_method", &self.auth_method)
            .field("channel", &self.channel)
            .finish()
    }
}

impl Default for ClientConfiguration {
    fn default() -> Self {
        ClientConfiguration {
            ssid: "".into(),
            bssid: None,
            auth_method: Default::default(),
            password: "".into(),
            channel: None,
        }
    }
}

#[derive(EnumSetType, Debug, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, Display, EnumMessage, EnumIter, EnumVariantNames, FromRepr)
)]
#[cfg_attr(feature = "use_numenum", derive(TryFromPrimitive))]
#[cfg_attr(feature = "use_numenum", repr(u8))]
pub enum Capability {
    #[cfg_attr(feature = "use_strum", strum(serialize = "client", message = "Client"))]
    Client,
    #[cfg_attr(
        feature = "use_strum",
        strum(serialize = "ap", message = "Access Point")
    )]
    AccessPoint,
    #[cfg_attr(
        feature = "use_strum",
        strum(serialize = "mixed", message = "Client & Access Point")
    )]
    Mixed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum Configuration {
    None,
    Client(ClientConfiguration),
    AccessPoint(AccessPointConfiguration),
    Mixed(ClientConfiguration, AccessPointConfiguration),
}

impl Configuration {
    pub fn as_client_conf_ref(&self) -> Option<&ClientConfiguration> {
        match self {
            Self::Client(client_conf) | Self::Mixed(client_conf, _) => Some(client_conf),
            _ => None,
        }
    }

    pub fn as_ap_conf_ref(&self) -> Option<&AccessPointConfiguration> {
        match self {
            Self::AccessPoint(ap_conf) | Self::Mixed(_, ap_conf) => Some(ap_conf),
            _ => None,
        }
    }

    pub fn as_client_conf_mut(&mut self) -> &mut ClientConfiguration {
        match self {
            Self::Client(client_conf) => client_conf,
            Self::Mixed(_, _) => {
                let prev = mem::replace(self, Self::None);
                match prev {
                    Self::Mixed(client_conf, _) => {
                        *self = Self::Client(client_conf);
                        self.as_client_conf_mut()
                    }
                    _ => unreachable!(),
                }
            }
            _ => {
                *self = Self::Client(Default::default());
                self.as_client_conf_mut()
            }
        }
    }

    pub fn as_ap_conf_mut(&mut self) -> &mut AccessPointConfiguration {
        match self {
            Self::AccessPoint(ap_conf) => ap_conf,
            Self::Mixed(_, _) => {
                let prev = mem::replace(self, Self::None);
                match prev {
                    Self::Mixed(_, ap_conf) => {
                        *self = Self::AccessPoint(ap_conf);
                        self.as_ap_conf_mut()
                    }
                    _ => unreachable!(),
                }
            }
            _ => {
                *self = Self::AccessPoint(Default::default());
                self.as_ap_conf_mut()
            }
        }
    }

    pub fn as_mixed_conf_mut(
        &mut self,
    ) -> (&mut ClientConfiguration, &mut AccessPointConfiguration) {
        match self {
            Self::Mixed(client_conf, ref mut ap_conf) => (client_conf, ap_conf),
            Self::AccessPoint(_) => {
                let prev = mem::replace(self, Self::None);
                match prev {
                    Self::AccessPoint(ap_conf) => {
                        *self = Self::Mixed(Default::default(), ap_conf);
                        self.as_mixed_conf_mut()
                    }
                    _ => unreachable!(),
                }
            }
            Self::Client(_) => {
                let prev = mem::replace(self, Self::None);
                match prev {
                    Self::Client(client_conf) => {
                        *self = Self::Mixed(client_conf, Default::default());
                        self.as_mixed_conf_mut()
                    }
                    _ => unreachable!(),
                }
            }
            _ => {
                *self = Self::Mixed(Default::default(), Default::default());
                self.as_mixed_conf_mut()
            }
        }
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration::None
    }
}

pub trait Wifi {
    type Error: Debug;

    fn get_capabilities(&self) -> Result<EnumSet<Capability>, Self::Error>;

    fn get_configuration(&self) -> Result<Configuration, Self::Error>;

    fn set_configuration(&mut self, conf: &Configuration) -> Result<(), Self::Error>;

    fn start(&mut self) -> Result<(), Self::Error>;
    fn stop(&mut self) -> Result<(), Self::Error>;

    fn connect(&mut self) -> Result<(), Self::Error>;
    fn disconnect(&mut self) -> Result<(), Self::Error>;

    fn is_started(&self) -> Result<bool, Self::Error>;
    fn is_connected(&self) -> Result<bool, Self::Error>;

    fn scan_n<const N: usize>(
        &mut self,
    ) -> Result<(heapless::Vec<AccessPointInfo, N>, usize), Self::Error>;

    #[cfg(feature = "alloc")]
    fn scan(&mut self) -> Result<alloc::vec::Vec<AccessPointInfo>, Self::Error>;
}

impl<W> Wifi for &mut W
where
    W: Wifi,
{
    type Error = W::Error;

    fn get_capabilities(&self) -> Result<EnumSet<Capability>, Self::Error> {
        (**self).get_capabilities()
    }

    fn get_configuration(&self) -> Result<Configuration, Self::Error> {
        (**self).get_configuration()
    }

    fn set_configuration(&mut self, conf: &Configuration) -> Result<(), Self::Error> {
        (*self).set_configuration(conf)
    }

    fn start(&mut self) -> Result<(), Self::Error> {
        (*self).start()
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        (*self).stop()
    }

    fn connect(&mut self) -> Result<(), Self::Error> {
        (*self).connect()
    }

    fn disconnect(&mut self) -> Result<(), Self::Error> {
        (*self).disconnect()
    }

    fn is_started(&self) -> Result<bool, Self::Error> {
        (**self).is_started()
    }

    fn is_connected(&self) -> Result<bool, Self::Error> {
        (**self).is_connected()
    }

    fn scan_n<const N: usize>(
        &mut self,
    ) -> Result<(heapless::Vec<AccessPointInfo, N>, usize), Self::Error> {
        (*self).scan_n()
    }

    #[cfg(feature = "alloc")]
    fn scan(&mut self) -> Result<alloc::vec::Vec<AccessPointInfo>, Self::Error> {
        (*self).scan()
    }
}

#[cfg(all(feature = "nightly", feature = "experimental"))]
mod asynch {
    use futures::Future;

    use super::*;

    pub trait Wifi {
        type Error: Debug;

        type GetCapabilitiesFuture<'a>: Future<Output = Result<EnumSet<Capability>, Self::Error>>
        where
            Self: 'a;

        type GetConfigurationFuture<'a>: Future<Output = Result<Configuration, Self::Error>>
        where
            Self: 'a;

        type SetConfigurationFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        type StartFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        type StopFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        type ConnectFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        type DisconnectFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        type IsStartedFuture<'a>: Future<Output = Result<bool, Self::Error>>
        where
            Self: 'a;

        type IsConnectedFuture<'a>: Future<Output = Result<bool, Self::Error>>
        where
            Self: 'a;

        type ScanNFuture<'a, const N: usize>: Future<
            Output = Result<(heapless::Vec<AccessPointInfo, N>, usize), Self::Error>,
        >
        where
            Self: 'a;

        #[cfg(feature = "alloc")]
        type ScanFuture<'a>: Future<Output = Result<alloc::vec::Vec<AccessPointInfo>, Self::Error>>
        where
            Self: 'a;

        fn get_capabilities(&self) -> Self::GetCapabilitiesFuture<'_>;

        fn get_configuration(&self) -> Self::GetConfigurationFuture<'_>;

        fn set_configuration(&mut self, conf: &Configuration) -> Self::SetConfigurationFuture<'_>;

        fn start(&mut self) -> Self::StartFuture<'_>;
        fn stop(&mut self) -> Self::StopFuture<'_>;

        fn connect(&mut self) -> Self::ConnectFuture<'_>;
        fn disconnect(&mut self) -> Self::DisconnectFuture<'_>;

        fn is_started(&self) -> Self::IsStartedFuture<'_>;
        fn is_connected(&self) -> Self::IsConnectedFuture<'_>;

        fn scan_n<const N: usize>(&mut self) -> Self::ScanNFuture<'_, N>;

        #[cfg(feature = "alloc")]
        fn scan(&mut self) -> Self::ScanFuture<'_>;
    }

    impl<W> Wifi for &mut W
    where
        W: Wifi,
    {
        type Error = W::Error;

        type GetCapabilitiesFuture<'a> = W::GetCapabilitiesFuture<'a>
        where
            Self: 'a;

        type GetConfigurationFuture<'a> = W::GetConfigurationFuture<'a>
        where
            Self: 'a;

        type SetConfigurationFuture<'a> = W::SetConfigurationFuture<'a>
        where
            Self: 'a;

        type StartFuture<'a> = W::StartFuture<'a>
        where
            Self: 'a;

        type StopFuture<'a> = W::StopFuture<'a>
        where
            Self: 'a;

        type ConnectFuture<'a> = W::ConnectFuture<'a>
        where
            Self: 'a;

        type DisconnectFuture<'a> = W::DisconnectFuture<'a>
        where
            Self: 'a;

        type IsStartedFuture<'a> = W::IsStartedFuture<'a>
        where
            Self: 'a;

        type IsConnectedFuture<'a> = W::IsConnectedFuture<'a>
        where
            Self: 'a;

        type ScanNFuture<'a, const N: usize> = W::ScanNFuture<'a, N>
        where
            Self: 'a;

        #[cfg(feature = "alloc")]
        type ScanFuture<'a> = W::ScanFuture<'a>
        where
            Self: 'a;

        fn get_capabilities(&self) -> Self::GetCapabilitiesFuture<'_> {
            (**self).get_capabilities()
        }

        fn get_configuration(&self) -> Self::GetConfigurationFuture<'_> {
            (**self).get_configuration()
        }

        fn set_configuration(&mut self, conf: &Configuration) -> Self::SetConfigurationFuture<'_> {
            (**self).set_configuration(conf)
        }

        fn start(&mut self) -> Self::StartFuture<'_> {
            (**self).start()
        }

        fn stop(&mut self) -> Self::StopFuture<'_> {
            (**self).stop()
        }

        fn connect(&mut self) -> Self::ConnectFuture<'_> {
            (**self).connect()
        }

        fn disconnect(&mut self) -> Self::DisconnectFuture<'_> {
            (**self).disconnect()
        }

        fn is_started(&self) -> Self::IsStartedFuture<'_> {
            (**self).is_started()
        }

        fn is_connected(&self) -> Self::IsConnectedFuture<'_> {
            (**self).is_connected()
        }

        fn scan_n<const N: usize>(&mut self) -> Self::ScanNFuture<'_, N> {
            (**self).scan_n()
        }

        #[cfg(feature = "alloc")]
        fn scan(&mut self) -> Self::ScanFuture<'_> {
            (**self).scan()
        }
    }
}
