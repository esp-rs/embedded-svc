use std::{collections, mem, vec};
use std::fmt::Debug;

use anyhow::*;

use serde::{Serialize, Deserialize};
use strum_macros::{EnumString, ToString, EnumMessage, EnumIter};

use async_trait::async_trait;

use crate::ipv4;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Debug, Serialize, Deserialize, EnumString, ToString, EnumMessage, EnumIter)]
pub enum AuthMethod {
    #[strum(serialize = "none", message = "None")]
    None,
    #[strum(serialize = "wep", message = "WEP")]
    WEP,
    #[strum(serialize = "wpa", message = "WPA")]
    WPA,
    #[strum(serialize = "wpa2personal", message = "WPA2 Personal")]
    WPA2Personal,
    #[strum(serialize = "wpa3personal", message = "WPA3 Personal")]
    WPA3Personal,
}

impl Default for AuthMethod {
    fn default() -> Self {
        AuthMethod::WPA2Personal
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, EnumString, ToString, EnumMessage, EnumIter)]
pub enum Protocol {
    #[strum(serialize = "p802d11b", message = "802.11B")]
    P802D11B,
    #[strum(serialize = "p802d11bg", message = "802.11BG")]
    P802D11BG,
    #[strum(serialize = "p802d11bgn", message = "802.11BGN")]
    P802D11BGN,
    #[strum(serialize = "p802d11bgnlr", message = "802.11BGNLR")]
    P802D11BGNLR,
    #[strum(serialize = "p802d11lr", message = "802.11LR")]
    P802D11LR,
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::P802D11BGN
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, EnumString, ToString, EnumMessage, EnumIter)]
pub enum SecondaryChannel { // TODO: Need to extend that for 5GHz
    #[strum(serialize = "none", message = "None")]
    None,
    #[strum(serialize = "above", message = "Above")]
    Above,
    #[strum(serialize = "below", message = "Below")]
    Below,
}

impl Default for SecondaryChannel {
    fn default() -> Self {
        SecondaryChannel::None
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccessPointInfo {
    pub ssid: String,
    pub bssid: [u8; 6],
    pub channel: u8,
    pub secondary_channel: SecondaryChannel,
    pub signal_strength: u8,
    pub protocols: collections::HashSet<Protocol>,
    pub auth_method: AuthMethod,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccessPointConfiguration {
    pub ssid: String,
    pub ssid_hidden: bool,
    pub channel: u8,
    pub secondary_channel: Option<u8>,
    pub protocols: collections::HashSet<Protocol>,
    pub auth_method: AuthMethod,
    pub password: String,
    pub max_connections: u16,
    pub ip_conf: Option<ipv4::RouterConfiguration>,
}

impl Default for AccessPointConfiguration {
    fn default() -> Self {
        Self {
            ssid: "iot-device".into(),
            ssid_hidden: false,
            channel: 1,
            secondary_channel: None,
            protocols: vec!(Protocol::P802D11B, Protocol::P802D11BG, Protocol::P802D11BGN).drain(..).collect(),
            auth_method: AuthMethod::None,
            password: "".into(),
            max_connections: 256,
            ip_conf: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientConfiguration {
    pub ssid: String,
    pub bssid: Option<[u8; 6]>,
    //pub protocol: Protocol,
    pub auth_method: AuthMethod,
    pub password: String,
    pub ip_conf: Option<ipv4::ClientConfiguration>,
}

impl ClientConfiguration {
    pub fn as_ip_conf_ref(&self) -> Option<&ipv4::ClientConfiguration> {
        self.ip_conf.as_ref()
    }

    pub fn as_ip_conf_mut(&mut self) -> &mut ipv4::ClientConfiguration {
        to_ip_conf(&mut self.ip_conf)
    }
}

fn to_ip_conf(ip_conf: &mut Option<ipv4::ClientConfiguration>) -> &mut ipv4::ClientConfiguration {
    if let Some(ip_conf) = ip_conf {
        return ip_conf
    }

    *ip_conf = Some(ipv4::ClientConfiguration::DHCP);
    to_ip_conf(ip_conf)
}

// impl Default for ClientConfiguration {
//     fn default() -> Self {
//         ClientConfiguration {
//             ssid: "".into(),
//             bssid: None,
//             auth_method: AuthMethod::WPA2Personal,
//             password: "".into(),
//             ip_conf: Some(Default::default()),
//         }
//     }
// }

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, EnumString, ToString, EnumMessage, EnumIter)]
pub enum Capability {
    #[strum(serialize = "client", message = "Client")]
    Client,
    #[strum(serialize = "ap", message = "Access Point")]
    AccessPoint,
    #[strum(serialize = "mixed", message = "Client & Access Point")]
    Mixed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
            _ => None
        }
    }

    pub fn as_ap_conf_ref(&self) -> Option<&AccessPointConfiguration> {
        match self {
            Self::AccessPoint(ap_conf) | Self::Mixed(_, ap_conf) => Some(ap_conf),
            _ => None
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
                    _ => unreachable!()
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
                    _ => unreachable!()
                }
            }
            _ => {
                *self = Self::AccessPoint(Default::default());
                self.as_ap_conf_mut()
            }
        }
    }

    pub fn as_mixed_conf_mut(&mut self) -> (&mut ClientConfiguration, &mut AccessPointConfiguration) {
        match self {
            Self::Mixed(client_conf, ref mut ap_conf) => (client_conf, ap_conf),
            Self::AccessPoint(_) => {
                let prev = mem::replace(self, Self::None);
                match prev {
                    Self::AccessPoint(ap_conf) => {
                        *self = Self::Mixed(Default::default(), ap_conf);
                        self.as_mixed_conf_mut()
                    }
                    _ => unreachable!()
                }
            }
            Self::Client(_) => {
                let prev = mem::replace(self, Self::None);
                match prev {
                    Self::Client(client_conf) => {
                        *self = Self::Mixed(client_conf, Default::default());
                        self.as_mixed_conf_mut()
                    }
                    _ => unreachable!()
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

pub trait TransitionalState {
    fn is_transitional(&self) -> bool;
    fn is_operating(&self) -> bool;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClientIpStatus {
    Disabled,
    Waiting,
    Done(ipv4::ClientSettings),
}

impl TransitionalState for ClientIpStatus {
    fn is_transitional(&self) -> bool {
        *self == ClientIpStatus::Waiting
    }

    fn is_operating(&self) -> bool {
        *self != ClientIpStatus::Disabled
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClientConnectionStatus {
    Disconnected,
    Connecting,
    Connected(ClientIpStatus),
}

impl TransitionalState for ClientConnectionStatus {
    fn is_transitional(&self) -> bool {
        *self == ClientConnectionStatus::Connecting || (if let ClientConnectionStatus::Connected(ips) = self {ips.is_transitional()} else {false})
    }

    fn is_operating(&self) -> bool {
        *self != ClientConnectionStatus::Disconnected
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClientStatus {
    Stopped,
    Starting,
    Started(ClientConnectionStatus),
}

impl TransitionalState for ClientStatus {
    fn is_transitional(&self) -> bool {
        *self == ClientStatus::Starting || (if let ClientStatus::Started(ccs) = self {ccs.is_transitional()} else {false})
    }

    fn is_operating(&self) -> bool {
        *self != ClientStatus::Stopped
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ApIpStatus {
    Disabled,
    Waiting,
    Done,
}

impl TransitionalState for ApIpStatus {
    fn is_transitional(&self) -> bool {
        *self == ApIpStatus::Waiting
    }

    fn is_operating(&self) -> bool {
        *self != ApIpStatus::Disabled
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ApStatus {
    Stopped,
    Starting,
    Started(ApIpStatus),
}

impl TransitionalState for ApStatus {
    fn is_transitional(&self) -> bool {
        *self == ApStatus::Starting || (if let ApStatus::Started(ips) = self {ips.is_transitional()} else {false})
    }

    fn is_operating(&self) -> bool {
        *self != ApStatus::Stopped
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Status(pub ClientStatus, pub ApStatus);

impl TransitionalState for Status {
    fn is_transitional(&self) -> bool {
        self.0.is_transitional() || self.1.is_transitional()
    }

    fn is_operating(&self) -> bool {
        self.0.is_operating() || self.1.is_operating()
    }
}

pub trait Wifi {
    fn get_capabilities(&self) -> Result<collections::HashSet<Capability>>;

    fn get_status(&self) -> Status;

    fn scan(&mut self) -> Result<vec::Vec<AccessPointInfo>>;

    fn get_configuration(&self) -> Result<Configuration>;
    fn set_configuration(&mut self, conf: &Configuration) -> Result<()>;
}

#[async_trait]
pub trait WifiAsync {
    async fn get_capabilities(&self) -> Result<collections::HashSet<Capability>>;

    async fn get_status(&self) -> Result<Status>;

    async fn scan(&mut self) -> Result<vec::Vec<AccessPointInfo>>;

    async fn get_configuration(&self) -> Result<Configuration>;
    async fn set_configuration(&mut self, conf: &Configuration) -> Result<()>;
}
