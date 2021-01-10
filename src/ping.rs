use std::{time::Duration, vec};

use anyhow::*;

use serde::{Serialize, Deserialize};

use crate::ipv4;

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Info {
    pub ttl: u8,
    pub elapsed_time: Duration,
    pub recv_len: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Reply {
    Timeout,
    Success(Info)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Summary {
    pub transmitted: u16,
    pub received: u16,
    pub time: Duration,
}

pub trait Ping {
    fn ping(&mut self, ip: ipv4::Ipv4Addr, conf: &Configuration) -> Result<vec::Vec<Reply>>;
    fn ping_summary(&mut self, ip: ipv4::Ipv4Addr, conf: &Configuration) -> Result<Summary>;
}