#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "experimental", feature(generic_associated_types))] // for http, http::client, http::server, ota and ghota
#![feature(backtrace)]

#[cfg(feature = "alloc")]
pub mod anyerror;
pub mod eth;
#[cfg(all(feature = "experimental", feature = "alloc"))]
pub mod http;
#[cfg(feature = "std")] // TODO: Lower requirements to "alloc"
pub mod httpd;
pub mod io;
pub mod ipv4;
pub mod mutex;
#[cfg(all(feature = "experimental", feature = "alloc"))]
pub mod ota;
pub mod ping;
#[cfg(feature = "alloc")]
pub mod storage;
#[cfg(feature = "alloc")]
pub mod wifi;

#[cfg(all(feature = "experimental", feature = "std", feature = "use_serde"))]
pub mod ghota;

#[cfg(all(feature = "alloc", feature = "use_serde"))]
pub mod edge_config;
