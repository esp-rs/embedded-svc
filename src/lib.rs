#![no_std]
#![allow(async_fn_in_trait)]

#[cfg(feature = "std")]
#[allow(unused_imports)]
#[macro_use]
extern crate std;

#[cfg(feature = "alloc")]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

#[cfg(all(feature = "defmt", feature = "log"))]
compile_error!("You must enable at most one of the following features: defmt, log");

pub mod channel;
pub mod eth;
pub mod http;
pub mod io;
pub mod ipv4;
pub mod log;
pub mod mqtt;
pub mod ota;
pub mod storage;
pub mod utils;
pub mod wifi;
pub mod ws;
