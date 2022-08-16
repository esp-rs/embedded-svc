use core::convert::TryFrom;
use core::fmt::Display;
use core::str::FromStr;

pub use no_std_net::*;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Hash))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct Mask(pub u8);

impl FromStr for Mask {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u8>()
            .map_err(|_| "Invalid subnet mask")
            .map_or_else(Err, |mask| {
                if (1..=32).contains(&mask) {
                    Ok(Mask(mask))
                } else {
                    Err("Mask should be a number between 1 and 32")
                }
            })
    }
}

impl Display for Mask {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<Ipv4Addr> for Mask {
    type Error = ();

    fn try_from(ip: Ipv4Addr) -> Result<Self, Self::Error> {
        let octets = ip.octets();
        let addr: u32 = ((octets[0] as u32 & 0xff) << 24)
            | ((octets[1] as u32 & 0xff) << 16)
            | ((octets[2] as u32 & 0xff) << 8)
            | (octets[3] as u32 & 0xff);

        if addr.leading_ones() + addr.trailing_zeros() == 32 {
            Ok(Mask(addr.leading_ones() as u8))
        } else {
            Err(())
        }
    }
}

impl From<Mask> for Ipv4Addr {
    fn from(mask: Mask) -> Self {
        let addr: u32 = ((1 << (32 - mask.0)) - 1) ^ 0xffffffffu32;

        let (a, b, c, d) = (
            ((addr >> 24) & 0xff) as u8,
            ((addr >> 16) & 0xff) as u8,
            ((addr >> 8) & 0xff) as u8,
            (addr & 0xff) as u8,
        );

        Ipv4Addr::new(a, b, c, d)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Hash))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct Subnet {
    pub gateway: Ipv4Addr,
    pub mask: Mask,
}

impl Display for Subnet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}/{}", self.gateway, self.mask)
    }
}

impl FromStr for Subnet {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split('/');
        if let Some(gateway_str) = split.next() {
            if let Some(mask_str) = split.next() {
                if split.next().is_none() {
                    if let Ok(gateway) = gateway_str.parse::<Ipv4Addr>() {
                        return mask_str.parse::<Mask>().map(|mask| Self { gateway, mask });
                    } else {
                        return Err("Invalid IP address format, expected XXX.XXX.XXX.XXX");
                    }
                }
            }
        }

        Err("Expected <gateway-ip-address>/<mask>")
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
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
                mask: Mask(24),
            },
            dns: Some(Ipv4Addr::new(8, 8, 8, 8)),
            secondary_dns: Some(Ipv4Addr::new(8, 8, 4, 4)),
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct DHCPClientSettings {
    pub hostname: Option<heapless::String<30>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum ClientConfiguration {
    DHCP(DHCPClientSettings),
    Fixed(ClientSettings),
}

impl ClientConfiguration {
    pub fn as_fixed_settings_ref(&self) -> Option<&ClientSettings> {
        match self {
            Self::Fixed(client_settings) => Some(client_settings),
            _ => None,
        }
    }

    pub fn as_fixed_settings_mut(&mut self) -> &mut ClientSettings {
        match self {
            Self::Fixed(client_settings) => client_settings,
            _ => {
                *self = ClientConfiguration::Fixed(Default::default());
                self.as_fixed_settings_mut()
            }
        }
    }
}

impl Default for ClientConfiguration {
    fn default() -> ClientConfiguration {
        ClientConfiguration::DHCP(Default::default())
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
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
                mask: Mask(24),
            },
            dhcp_enabled: true,
            dns: Some(Ipv4Addr::new(8, 8, 8, 8)),
            secondary_dns: Some(Ipv4Addr::new(8, 8, 4, 4)),
        }
    }
}
