pub use std::net::Ipv4Addr;

use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Subnet {
    pub gateway: Ipv4Addr,
    pub mask: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClientSettings {
    pub ip: Ipv4Addr,
    pub subnet: Subnet,
    pub dns: Option<Ipv4Addr>,
    pub secondary_dns: Option<Ipv4Addr>,
}

impl Default for ClientSettings {
    fn default() -> ClientSettings {
        ClientSettings {
            ip: Ipv4Addr::new(192, 168, 71, 200),
            subnet: Subnet {
                gateway: Ipv4Addr::new(192, 168, 71, 1),
                mask: 24,
            },
            dns: Some(Ipv4Addr::new(8, 8, 8, 8)),
            secondary_dns: Some(Ipv4Addr::new(8, 8, 4, 4)),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClientConfiguration {
    DHCP,
    Fixed(ClientSettings),
}

impl Default for ClientConfiguration {
    fn default() -> ClientConfiguration {
        ClientConfiguration::DHCP
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouterConfiguration {
    pub subnet: Subnet,
    pub dhcp_enabled: bool,
    pub dns: Option<Ipv4Addr>,
    pub secondary_dns: Option<Ipv4Addr>,
}

impl Default for RouterConfiguration {
    fn default() -> RouterConfiguration {
        RouterConfiguration {
            subnet: Subnet {
                gateway: Ipv4Addr::new(192, 168, 71, 1),
                mask: 24,
            },
            dhcp_enabled: true,
            dns: Some(Ipv4Addr::new(8, 8, 8, 8)),
            secondary_dns: Some(Ipv4Addr::new(8, 8, 4, 4)),
        }
    }
}
