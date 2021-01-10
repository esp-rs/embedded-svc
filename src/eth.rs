use std::collections;
use std::fmt::Debug;

use anyhow::*;

use serde::{Serialize, Deserialize};

use crate::ipv4;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum OperationMode {
    Client,
    Router,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Configuration {
    None,
    Client(ipv4::ClientConfiguration),
    Router(ipv4::RouterConfiguration),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientIpStatus {
    Disabled,
    Waiting,
    Done(ipv4::ClientSettings),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientConnectionStatus {
    Disconnected,
    Connecting,
    Connected(ClientIpStatus),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientStatus {
    Stopped,
    Starting,
    Started(ClientConnectionStatus),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RouterIpStatus {
    Disabled,
    Waiting,
    Done,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RouterStatus {
    Stopped,
    Starting,
    Started(RouterIpStatus),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Status(pub ClientStatus, pub RouterStatus);

pub trait Eth {
    fn get_supported_operation_modes(&self) -> Result<collections::HashSet<OperationMode>>;

    fn get_status(&self) -> Status;

    fn get_configuration(&self) -> Result<Configuration>;
    fn set_configuration(&mut self, conf: &Configuration) -> Result<()>;
}
