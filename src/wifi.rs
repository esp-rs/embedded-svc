use core::fmt::Debug;
use core::mem;

#[cfg(feature = "alloc")]
extern crate alloc;

use enumset::*;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "use_strum")]
use strum_macros::{Display, EnumIter, EnumMessage, EnumString};

#[cfg(feature = "use_numenum")]
use num_enum::TryFromPrimitive;

use crate::errors::Errors;
use crate::ipv4;

#[derive(EnumSetType, Debug, PartialOrd)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, Display, EnumMessage, EnumIter)
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
    derive(EnumString, Display, EnumMessage, EnumIter)
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
    derive(EnumString, Display, EnumMessage, EnumIter)
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

#[derive(Copy /*TODO: Not ideal*/, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct AccessPointInfo<'a> {
    pub ssid: &'a str,
    pub bssid: [u8; 6],
    pub channel: u8,
    pub secondary_channel: SecondaryChannel,
    pub signal_strength: u8,
    pub protocols: EnumSet<Protocol>,
    pub auth_method: AuthMethod,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct AccessPointConfiguration<S> {
    pub ssid: S,
    pub ssid_hidden: bool,
    pub channel: u8,
    pub secondary_channel: Option<u8>,
    pub protocols: EnumSet<Protocol>,
    pub auth_method: AuthMethod,
    pub password: S,
    pub max_connections: u16,
    pub ip_conf: Option<ipv4::RouterConfiguration>,
}

impl<S> AccessPointConfiguration<S> {
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

impl<S: for<'a> From<&'a str>> Default for AccessPointConfiguration<S> {
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
pub struct ClientConfiguration<S> {
    pub ssid: S,
    pub bssid: Option<[u8; 6]>,
    //pub protocol: Protocol,
    pub auth_method: AuthMethod,
    pub password: S,
    pub channel: Option<u8>,
    pub ip_conf: Option<ipv4::ClientConfiguration<S>>,
}

impl<S> ClientConfiguration<S> {
    pub fn as_ip_conf_ref(&self) -> Option<&ipv4::ClientConfiguration<S>> {
        self.ip_conf.as_ref()
    }

    pub fn as_ip_conf_mut(&mut self) -> &mut ipv4::ClientConfiguration<S> {
        Self::to_ip_conf(&mut self.ip_conf)
    }

    fn to_ip_conf(
        ip_conf: &mut Option<ipv4::ClientConfiguration<S>>,
    ) -> &mut ipv4::ClientConfiguration<S> {
        if let Some(ip_conf) = ip_conf {
            return ip_conf;
        }

        *ip_conf = Some(ipv4::ClientConfiguration::DHCP(Default::default()));
        Self::to_ip_conf(ip_conf)
    }
}

impl<S: for<'a> From<&'a str>> Default for ClientConfiguration<S> {
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
    derive(EnumString, Display, EnumMessage, EnumIter)
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
pub enum Configuration<S> {
    None,
    Client(ClientConfiguration<S>),
    AccessPoint(AccessPointConfiguration<S>),
    Mixed(ClientConfiguration<S>, AccessPointConfiguration<S>),
}

impl<S> Configuration<S> {
    pub fn as_client_conf_ref(&self) -> Option<&ClientConfiguration<S>> {
        match self {
            Self::Client(client_conf) | Self::Mixed(client_conf, _) => Some(client_conf),
            _ => None,
        }
    }

    pub fn as_ap_conf_ref(&self) -> Option<&AccessPointConfiguration<S>> {
        match self {
            Self::AccessPoint(ap_conf) | Self::Mixed(_, ap_conf) => Some(ap_conf),
            _ => None,
        }
    }

    pub fn as_client_conf_mut(&mut self) -> &mut ClientConfiguration<S>
    where
        S: for<'a> From<&'a str>,
    {
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

    pub fn as_ap_conf_mut(&mut self) -> &mut AccessPointConfiguration<S>
    where
        S: for<'a> From<&'a str>,
    {
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
    ) -> (
        &mut ClientConfiguration<S>,
        &mut AccessPointConfiguration<S>,
    )
    where
        S: for<'a> From<&'a str>,
    {
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

impl<S> Default for Configuration<S> {
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

pub trait Wifi: Errors {
    fn get_capabilities(&self) -> Result<EnumSet<Capability>, Self::Error>;

    fn get_status(&self) -> Status;

    //fn scan_n<const N: usize = 20>(&mut self) -> Result<([AccessPointInfo; N], usize), Self::Error>;

    #[cfg(not(feature = "alloc"))]
    fn scan_fill<'a>(
        &'a mut self,
        access_points: &'a mut [AccessPointInfo<'a>],
    ) -> Result<(&'a [AccessPointInfo<'b>], usize), Self::Error>;

    #[cfg(feature = "alloc")]
    fn scan_fill<'a>(
        &'a mut self,
        access_points: &'a mut [AccessPointInfo<'a>],
    ) -> Result<(&'a [AccessPointInfo<'a>], usize), Self::Error> {
        let result = self.scan()?;

        let len = usize::min(access_points.len(), result.len());

        access_points[0..len].clone_from_slice(&result[0..len]);

        Ok((&access_points[0..len], result.len()))
    }

    #[cfg(feature = "alloc")]
    fn scan(&mut self) -> Result<alloc::vec::Vec<AccessPointInfo<'_>>, Self::Error>;

    fn get_configuration<'a>(&'a self) -> Result<Configuration<&'a str>, Self::Error>;
    fn set_configuration(&mut self, conf: &Configuration<&'_ str>) -> Result<(), Self::Error>;
}
