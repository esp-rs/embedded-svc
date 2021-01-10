use std::{collections, vec};
use std::fmt::Debug;

use anyhow::*;

use serde::{Serialize, Deserialize};
use async_trait::async_trait;

use crate::ipv4;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Debug, Serialize, Deserialize)]
pub enum AuthMethod {
    None,
    WEP,
    WPA,
    WPA2Personal,
    WPA3Personal,
}

impl Default for AuthMethod {
    fn default() -> AuthMethod {
        AuthMethod::WPA2Personal
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Protocol {
    P802D11B,
    P802D11BG,
    P802D11BGN,
    P802D11BGNLR,
    P802D11LR,
}

impl Default for Protocol {
    fn default() -> Protocol {
        Protocol::P802D11BGN
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum SecondaryChannel { // TODO: Need to extend that for 5GHz
    None,
    Above,
    Below,
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
        AccessPointConfiguration {
            ssid: "espressif".into(),
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

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum OperationMode {
    Client,
    AccessPoint,
    Mixed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Configuration {
    None,
    Client(ClientConfiguration),
    AccessPoint(AccessPointConfiguration),
    Mixed(ClientConfiguration, AccessPointConfiguration),
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
    fn get_supported_operation_modes(&self) -> Result<collections::HashSet<OperationMode>>;

    fn get_status(&self) -> Status;

    fn scan(&mut self) -> Result<vec::Vec<AccessPointInfo>>;

    fn get_configuration(&self) -> Result<Configuration>;
    fn set_configuration(&mut self, conf: &Configuration) -> Result<()>;
}

#[async_trait]
pub trait WifiAsync {
    async fn get_supported_operation_modes(&self) -> Result<collections::HashSet<OperationMode>>;

    async fn get_status(&self) -> Status;

    async fn scan(&mut self) -> Result<vec::Vec<AccessPointInfo>>;

    async fn get_configuration(&self) -> Result<Configuration>;
    async fn set_configuration(&mut self, conf: &Configuration) -> Result<()>;
}
