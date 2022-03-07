#![cfg_attr(not(feature = "std"), no_std)]
#![feature(generic_associated_types)] // For mutex, http, http::client, http::server, ota and ghota
#![cfg_attr(feature = "experimental", feature(type_alias_impl_trait))] // For the Sender/Receiver adapters

#[cfg(feature = "alloc")]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

pub mod channel;
pub mod errors;
pub mod eth;
pub mod event_bus;
#[cfg(all(feature = "experimental", feature = "alloc"))]
pub mod http;
#[cfg(feature = "std")] // TODO: Lower requirements to "alloc"
pub mod httpd;
pub mod io;
pub mod ipv4;
#[cfg(feature = "alloc")]
pub mod mqtt;
pub mod mutex;
#[cfg(all(feature = "experimental", feature = "alloc"))]
pub mod ota;
pub mod ping;
#[cfg(feature = "alloc")]
pub mod storage;
pub mod sys_time;
pub mod timer;
pub mod unblocker;
pub mod utils;
#[cfg(feature = "alloc")]
pub mod wifi;
#[cfg(feature = "experimental")]
pub mod ws;
