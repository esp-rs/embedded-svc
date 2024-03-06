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
#[derive(Default)]
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
    #[default]
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

#[derive(EnumSetType, Debug, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, Display, EnumMessage, EnumIter, EnumVariantNames, FromRepr)
)]
#[cfg_attr(feature = "use_numenum", derive(TryFromPrimitive))]
#[cfg_attr(feature = "use_numenum", repr(u8))]
#[derive(Default)]
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
    #[default]
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

#[derive(EnumSetType, Debug, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, Display, EnumMessage, EnumIter, EnumVariantNames, FromRepr)
)]
#[cfg_attr(feature = "use_numenum", derive(TryFromPrimitive))]
#[cfg_attr(feature = "use_numenum", repr(u8))]
#[derive(Default)]
pub enum SecondaryChannel {
    // TODO: Need to extend that for 5GHz
    #[cfg_attr(feature = "use_strum", strum(serialize = "none", message = "None"))]
    #[default]
    None,
    #[cfg_attr(feature = "use_strum", strum(serialize = "above", message = "Above"))]
    Above,
    #[cfg_attr(feature = "use_strum", strum(serialize = "below", message = "Below"))]
    Below,
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
    pub auth_method: Option<AuthMethod>,
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
            ssid: "iot-device".try_into().unwrap(),
            ssid_hidden: false,
            channel: 1,
            secondary_channel: None,
            protocols: Protocol::P802D11B | Protocol::P802D11BG | Protocol::P802D11BGN,
            auth_method: AuthMethod::None,
            password: heapless::String::new(),
            max_connections: 255,
        }
    }
}

/// Configuration for wifi in STA mode
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct ClientConfiguration {
    /// SSID of the target AP
    pub ssid: heapless::String<32>,
    /// BSSID of the target AP
    pub bssid: Option<[u8; 6]>,
    //pub protocol: Protocol,
    pub auth_method: AuthMethod,
    pub password: heapless::String<64>,
    /// The expected Channel of the target AP
    ///
    /// Connecting might be quicker when the client starts its scan
    /// at the channel the target AP is on.
    pub channel: Option<u8>,
    /// The scan method to use when searching for the target AP
    pub scan_method: ScanMethod,
    /// Protected Management Frame configuration
    pub pmf_cfg: PmfConfiguration,
}

impl Debug for ClientConfiguration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ClientConfiguration")
            .field("ssid", &self.ssid)
            .field("bssid", &self.bssid)
            .field("auth_method", &self.auth_method)
            .field("channel", &self.channel)
            .field("scan_method", &self.scan_method)
            .field("pmf_cfg", &self.pmf_cfg)
            .finish()
    }
}

impl Default for ClientConfiguration {
    fn default() -> Self {
        ClientConfiguration {
            ssid: heapless::String::new(),
            bssid: None,
            auth_method: Default::default(),
            password: heapless::String::new(),
            channel: None,
            scan_method: ScanMethod::default(),
            pmf_cfg: PmfConfiguration::default(),
        }
    }
}

/// Protected Management Frame configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, Display, EnumMessage, EnumIter, EnumVariantNames)
)]
pub enum PmfConfiguration {
    /// No support for PMF will be advertized (default)
    #[default]
    #[cfg_attr(
        feature = "use_strum",
        strum(
            serialize = "not_capable",
            serialize = "pmf_disabled",
            to_string = "PMF Disabled",
            message = "Don't advertise PMF capabilities",
        )
    )]
    NotCapable,
    /// Advertize PMF support and wether PMF is required or not
    #[cfg_attr(
        feature = "use_strum",
        strum(
            serialize = "capable",
            to_string = "PMF enabled",
            message = "Advertise PMF capabilities",
        )
    )]
    Capable { required: bool },
}
impl PmfConfiguration {
    /// PMF configuration with PMF strictly required
    pub fn new_required() -> Self {
        PmfConfiguration::Capable { required: true }
    }
    /// PMF configuration with PMF optional but available
    pub fn new_pmf_optional() -> Self {
        PmfConfiguration::Capable { required: true }
    }
}

/// The scan method to use when connecting to an AP
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, Display, EnumMessage, EnumIter, EnumVariantNames)
)]
#[non_exhaustive]
pub enum ScanMethod {
    /// Scan every channel and connect according to [ScanSortMethod] (default)
    #[cfg_attr(
        feature = "use_strum",
        strum(
            serialize = "complete_scan",
            to_string = "Complete Scan",
            message = "Do a complete scan",
            detailed_message = "Scan all APs and sort by a criteria"
        )
    )]
    CompleteScan(ScanSortMethod),
    /// Connect to the first found AP and stop scanning
    #[cfg_attr(
        feature = "use_strum",
        strum(
            serialize = "fast_scan",
            to_string = "Fast Scan",
            message = "Do a fast scan",
            detailed_message = "Connect to the first matching AP"
        )
    )]
    FastScan,
}
impl Default for ScanMethod {
    fn default() -> Self {
        Self::CompleteScan(ScanSortMethod::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, Display, EnumMessage, EnumIter, EnumVariantNames)
)]
#[non_exhaustive]
pub enum ScanSortMethod {
    /// Sort by signal strength (default)
    #[default]
    #[cfg_attr(
        feature = "use_strum",
        strum(
            serialize = "signal_strength",
            serialize = "signal",
            to_string = "Signal Strength",
            message = "Sort by signal strength"
        )
    )]
    Signal,
    /// Sort by Security
    #[cfg_attr(
        feature = "use_strum",
        strum(
            serialize = "security",
            to_string = "Security",
            message = "Sort by security"
        )
    )]
    Security,
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
#[derive(Default)]
pub enum Configuration {
    #[default]
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

pub mod asynch {
    use super::*;

    pub trait Wifi {
        type Error: Debug;

        async fn get_capabilities(&self) -> Result<EnumSet<Capability>, Self::Error>;

        async fn get_configuration(&self) -> Result<Configuration, Self::Error>;

        async fn set_configuration(&mut self, conf: &Configuration) -> Result<(), Self::Error>;

        async fn start(&mut self) -> Result<(), Self::Error>;
        async fn stop(&mut self) -> Result<(), Self::Error>;

        async fn connect(&mut self) -> Result<(), Self::Error>;
        async fn disconnect(&mut self) -> Result<(), Self::Error>;

        async fn is_started(&self) -> Result<bool, Self::Error>;
        async fn is_connected(&self) -> Result<bool, Self::Error>;

        async fn scan_n<const N: usize>(
            &mut self,
        ) -> Result<(heapless::Vec<AccessPointInfo, N>, usize), Self::Error>;

        #[cfg(feature = "alloc")]
        async fn scan(&mut self) -> Result<alloc::vec::Vec<AccessPointInfo>, Self::Error>;
    }

    impl<W> Wifi for &mut W
    where
        W: Wifi,
    {
        type Error = W::Error;

        async fn get_capabilities(&self) -> Result<EnumSet<Capability>, Self::Error> {
            (**self).get_capabilities().await
        }

        async fn get_configuration(&self) -> Result<Configuration, Self::Error> {
            (**self).get_configuration().await
        }

        async fn set_configuration(&mut self, conf: &Configuration) -> Result<(), Self::Error> {
            (**self).set_configuration(conf).await
        }

        async fn start(&mut self) -> Result<(), Self::Error> {
            (**self).start().await
        }

        async fn stop(&mut self) -> Result<(), Self::Error> {
            (**self).stop().await
        }

        async fn connect(&mut self) -> Result<(), Self::Error> {
            (**self).connect().await
        }

        async fn disconnect(&mut self) -> Result<(), Self::Error> {
            (**self).disconnect().await
        }

        async fn is_started(&self) -> Result<bool, Self::Error> {
            (**self).is_started().await
        }

        async fn is_connected(&self) -> Result<bool, Self::Error> {
            (**self).is_connected().await
        }

        async fn scan_n<const N: usize>(
            &mut self,
        ) -> Result<(heapless::Vec<AccessPointInfo, N>, usize), Self::Error> {
            (**self).scan_n::<N>().await
        }

        #[cfg(feature = "alloc")]
        async fn scan(&mut self) -> Result<alloc::vec::Vec<AccessPointInfo>, Self::Error> {
            (**self).scan().await
        }
    }
}
