use core::fmt::Debug;
use core::mem;

extern crate alloc;
use alloc::boxed::Box;

use enumset::*;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "use_strum")]
use strum_macros::{EnumIter, EnumMessage, EnumString, ToString};

#[cfg(feature = "use_numenum")]
use num_enum::TryFromPrimitive;

use async_trait::async_trait;

use crate::ipv4;

#[derive(EnumSetType, Debug, PartialOrd)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, ToString, EnumMessage, EnumIter)
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
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, ToString, EnumMessage, EnumIter)
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
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, ToString, EnumMessage, EnumIter)
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

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct AccessPointInfo {
    pub ssid: alloc::string::String,
    pub bssid: [u8; 6],
    pub channel: u8,
    pub secondary_channel: SecondaryChannel,
    pub signal_strength: u8,
    pub protocols: EnumSet<Protocol>,
    pub auth_method: AuthMethod,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct AccessPointConfiguration {
    pub ssid: alloc::string::String,
    pub ssid_hidden: bool,
    pub channel: u8,
    pub secondary_channel: Option<u8>,
    pub protocols: EnumSet<Protocol>,
    pub auth_method: AuthMethod,
    pub password: alloc::string::String,
    pub max_connections: u16,
    pub ip_conf: Option<ipv4::RouterConfiguration>,
}

impl AccessPointConfiguration {
    pub fn as_ip_conf_ref(&self) -> Option<&ipv4::RouterConfiguration> {
        self.ip_conf.as_ref()
    }

    pub fn as_ip_conf_mut(&mut self) -> &mut ipv4::RouterConfiguration {
        Self::to_ip_conf(&mut self.ip_conf)
    }

    fn to_ip_conf(
        ip_conf: &mut Option<ipv4::RouterConfiguration>,
    ) -> &mut ipv4::RouterConfiguration {
        if let Some(ip_conf) = ip_conf {
            return ip_conf;
        }

        *ip_conf = Some(Default::default());
        Self::to_ip_conf(ip_conf)
    }
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
            ip_conf: Some(Default::default()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct ClientConfiguration {
    pub ssid: alloc::string::String,
    pub bssid: Option<[u8; 6]>,
    //pub protocol: Protocol,
    pub auth_method: AuthMethod,
    pub password: alloc::string::String,
    pub channel: Option<u8>,
    pub ip_conf: Option<ipv4::ClientConfiguration>,
}

impl ClientConfiguration {
    pub fn as_ip_conf_ref(&self) -> Option<&ipv4::ClientConfiguration> {
        self.ip_conf.as_ref()
    }

    pub fn as_ip_conf_mut(&mut self) -> &mut ipv4::ClientConfiguration {
        Self::to_ip_conf(&mut self.ip_conf)
    }

    fn to_ip_conf(
        ip_conf: &mut Option<ipv4::ClientConfiguration>,
    ) -> &mut ipv4::ClientConfiguration {
        if let Some(ip_conf) = ip_conf {
            return ip_conf;
        }

        *ip_conf = Some(ipv4::ClientConfiguration::DHCP(Default::default()));
        Self::to_ip_conf(ip_conf)
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
            ip_conf: Some(Default::default()),
        }
    }
}

#[derive(EnumSetType, Debug, PartialOrd)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, ToString, EnumMessage, EnumIter)
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

#[derive(Clone, Debug, PartialEq)]
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

pub trait TransitionalState<T> {
    fn is_transitional(&self) -> bool;
    fn is_operating(&self) -> bool;
    fn get_operating(&self) -> Option<&T>;
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum ClientIpStatus {
    Disabled,
    Waiting,
    Done(ipv4::ClientSettings),
}

impl TransitionalState<ipv4::ClientSettings> for ClientIpStatus {
    fn is_transitional(&self) -> bool {
        *self == ClientIpStatus::Waiting
    }

    fn is_operating(&self) -> bool {
        *self != ClientIpStatus::Disabled
    }

    fn get_operating(&self) -> Option<&ipv4::ClientSettings> {
        if let ClientIpStatus::Done(ref settings) = *self {
            Some(settings)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum ClientConnectionStatus {
    Disconnected,
    Connecting,
    Connected(ClientIpStatus),
}

impl TransitionalState<ClientIpStatus> for ClientConnectionStatus {
    fn is_transitional(&self) -> bool {
        *self == ClientConnectionStatus::Connecting
            || (if let ClientConnectionStatus::Connected(ips) = self {
                ips.is_transitional()
            } else {
                false
            })
    }

    fn is_operating(&self) -> bool {
        *self != ClientConnectionStatus::Disconnected
    }

    fn get_operating(&self) -> Option<&ClientIpStatus> {
        if let ClientConnectionStatus::Connected(ref settings) = *self {
            Some(settings)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum ClientStatus {
    Stopped,
    Starting,
    Started(ClientConnectionStatus),
}

impl TransitionalState<ClientConnectionStatus> for ClientStatus {
    fn is_transitional(&self) -> bool {
        *self == ClientStatus::Starting
            || (if let ClientStatus::Started(ccs) = self {
                ccs.is_transitional()
            } else {
                false
            })
    }

    fn is_operating(&self) -> bool {
        *self != ClientStatus::Stopped
    }

    fn get_operating(&self) -> Option<&ClientConnectionStatus> {
        if let ClientStatus::Started(ref settings) = *self {
            Some(settings)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum ApIpStatus {
    Disabled,
    Waiting,
    Done,
}

impl TransitionalState<()> for ApIpStatus {
    fn is_transitional(&self) -> bool {
        *self == ApIpStatus::Waiting
    }

    fn is_operating(&self) -> bool {
        *self != ApIpStatus::Disabled
    }

    fn get_operating(&self) -> Option<&()> {
        if let ApIpStatus::Done = *self {
            Some(&())
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum ApStatus {
    Stopped,
    Starting,
    Started(ApIpStatus),
}

impl TransitionalState<ApIpStatus> for ApStatus {
    fn is_transitional(&self) -> bool {
        *self == ApStatus::Starting
            || (if let ApStatus::Started(ips) = self {
                ips.is_transitional()
            } else {
                false
            })
    }

    fn is_operating(&self) -> bool {
        *self != ApStatus::Stopped
    }

    fn get_operating(&self) -> Option<&ApIpStatus> {
        if let ApStatus::Started(ref settings) = *self {
            Some(settings)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct Status(pub ClientStatus, pub ApStatus);

impl Status {
    pub fn is_transitional(&self) -> bool {
        self.0.is_transitional() || self.1.is_transitional()
    }

    pub fn is_operating(&self) -> bool {
        self.0.is_operating() || self.1.is_operating()
    }
}

pub trait Wifi {
    #[cfg(not(feature = "std"))]
    type Error: core::fmt::Debug + core::fmt::Display;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;

    fn get_capabilities(&self) -> Result<EnumSet<Capability>, Self::Error>;

    fn get_status(&self) -> Status;

    //fn scan_n<const N: usize = 20>(&mut self) -> Result<([AccessPointInfo; N], usize), Self::Error>;

    #[cfg(not(feature = "alloc"))]
    fn scan_fill(&mut self, access_points: &mut [AccessPointInfo]) -> Result<usize, Self::Error>;

    #[cfg(feature = "alloc")]
    fn scan_fill(&mut self, access_points: &mut [AccessPointInfo]) -> Result<usize, Self::Error> {
        let result = self.scan()?;

        let len = usize::min(access_points.len(), result.len());

        access_points[0..len].clone_from_slice(&result[0..len]);

        Ok(result.len())
    }

    #[cfg(feature = "alloc")]
    fn scan(&mut self) -> Result<alloc::vec::Vec<AccessPointInfo>, Self::Error>;

    fn get_configuration(&self) -> Result<Configuration, Self::Error>;
    fn set_configuration(&mut self, conf: &Configuration) -> Result<(), Self::Error>;
}

#[async_trait]
pub trait WifiAsync {
    #[cfg(not(feature = "std"))]
    type Error: core::fmt::Debug + core::fmt::Display;

    #[cfg(feature = "std")]
    type Error: std::error::Error + Send + Sync + 'static;

    async fn get_capabilities(&self) -> Result<EnumSet<Capability>, Self::Error>;

    async fn get_status(&self) -> Result<Status, Self::Error>;

    //async fn scan_n<const N: usize = 20>(&mut self) -> Result<([AccessPointInfo; N], usize)>;

    #[cfg(not(feature = "alloc"))]
    async fn scan_fill(
        &mut self,
        access_points: &mut [AccessPointInfo],
    ) -> Result<usize, Self::Error>
    where
        Self: Sized;

    #[cfg(feature = "alloc")]
    async fn scan_fill(
        &mut self,
        access_points: &mut [AccessPointInfo],
    ) -> Result<usize, Self::Error>
    where
        Self: Sized,
    {
        let result = self.scan().await?;

        let len = usize::min(access_points.len(), result.len());

        access_points[0..len].clone_from_slice(&result[0..len]);

        Ok(result.len())
    }

    #[cfg(feature = "alloc")]
    async fn scan(&mut self) -> Result<alloc::vec::Vec<AccessPointInfo>, Self::Error>;

    async fn get_configuration(&self) -> Result<Configuration, Self::Error>;
    async fn set_configuration(&mut self, conf: &Configuration) -> Result<(), Self::Error>;
}
