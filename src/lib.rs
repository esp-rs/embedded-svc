#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "nightly", feature(generic_associated_types))]
#![cfg_attr(feature = "nightly", feature(type_alias_impl_trait))]

#[cfg(feature = "alloc")]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod channel;
pub mod errors;
pub mod eth;
pub mod event_bus;
pub mod executor;
#[cfg(feature = "experimental")]
pub mod http;
#[cfg(feature = "std")]
pub mod httpd; // TODO: Retire
pub mod io;
pub mod ipv4;
pub mod mqtt;
pub mod mutex;
#[cfg(feature = "experimental")]
pub mod ota;
pub mod ping;
pub mod storage;
pub mod sys_time;
pub mod timer;
pub mod utils;
pub mod wifi;
#[cfg(feature = "experimental")]
pub mod ws;
