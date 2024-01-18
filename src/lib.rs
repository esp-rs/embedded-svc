#![cfg_attr(not(feature = "std"), no_std)]
#![allow(async_fn_in_trait)]

#[cfg(feature = "alloc")]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

#[cfg(all(feature = "defmt", feature = "log"))]
compile_error!("You must enable at most one of the following features: defmt, log");

pub mod eth;
pub mod event_bus;
pub mod http;
pub mod io;
pub mod ipv4;
pub mod log;
pub mod mqtt;
pub mod ota;
pub mod ping;
pub mod storage;
pub mod sys_time;
pub mod timer;
pub mod utils;
pub mod wifi;
pub mod ws;
