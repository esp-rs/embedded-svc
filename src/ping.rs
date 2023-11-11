use core::fmt::Debug;
use core::time::Duration;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

use crate::ipv4;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct Configuration {
    pub count: u32,
    pub interval: Duration,
    pub timeout: Duration,
    pub data_size: u32,
    pub tos: u8,
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            count: 5,
            interval: Duration::from_secs(1),
            timeout: Duration::from_secs(1),
            data_size: 56,
            tos: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct Info {
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub addr: ipv4::Ipv4Addr,
    pub seqno: u32,
    pub ttl: u8,
    pub elapsed_time: Duration,
    pub recv_len: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum Reply {
    Timeout,
    Success(Info),
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct Summary {
    pub transmitted: u32,
    pub received: u32,
    pub time: Duration,
}

pub trait Ping {
    type Error: Debug;

    fn ping(&mut self, ip: ipv4::Ipv4Addr, conf: &Configuration) -> Result<Summary, Self::Error>;

    fn ping_details<F: FnMut(&Summary, &Reply) + Send + 'static>(
        &mut self,
        ip: ipv4::Ipv4Addr,
        conf: &Configuration,
        reply_callback: F,
    ) -> Result<Summary, Self::Error>;
}

impl<P> Ping for &mut P
where
    P: Ping,
{
    type Error = P::Error;

    fn ping(&mut self, ip: ipv4::Ipv4Addr, conf: &Configuration) -> Result<Summary, Self::Error> {
        (*self).ping(ip, conf)
    }

    fn ping_details<F: FnMut(&Summary, &Reply) + Send + 'static>(
        &mut self,
        ip: ipv4::Ipv4Addr,
        conf: &Configuration,
        reply_callback: F,
    ) -> Result<Summary, Self::Error> {
        (*self).ping_details(ip, conf, reply_callback)
    }
}
#[cfg(feature = "nightly")]
pub mod asynch {
    use core::fmt::Debug;

    use crate::ipv4;

    pub use super::{Configuration, Reply, Summary};

    pub trait Ping {
        type Error: Debug;

        async fn ping(
            &mut self,
            ip: ipv4::Ipv4Addr,
            conf: &Configuration,
        ) -> Result<Summary, Self::Error>;

        async fn ping_details<F: FnMut(&Summary, &Reply) + Send + 'static>(
            &mut self,
            ip: ipv4::Ipv4Addr,
            conf: &Configuration,
            reply_callback: F,
        ) -> Result<Summary, Self::Error>;
    }

    impl<P> Ping for &mut P
    where
        P: Ping,
    {
        type Error = P::Error;

        async fn ping(
            &mut self,
            ip: ipv4::Ipv4Addr,
            conf: &Configuration,
        ) -> Result<Summary, Self::Error> {
            (*self).ping(ip, conf).await
        }

        async fn ping_details<F: FnMut(&Summary, &Reply) + Send + 'static>(
            &mut self,
            ip: ipv4::Ipv4Addr,
            conf: &Configuration,
            reply_callback: F,
        ) -> Result<Summary, Self::Error> {
            (*self).ping_details(ip, conf, reply_callback).await
        }
    }
}
