#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
pub mod anyerror;
pub mod eth;
#[cfg(feature = "std")] // TODO: Lower requirements to "alloc"
pub mod httpd;
pub mod ipv4;
pub mod mutex;
pub mod ping;
#[cfg(feature = "alloc")] // TODO: Expose a subset which does not require "alloc"
pub mod storage;
#[cfg(feature = "alloc")] // TODO: Expose a subset which does not require "alloc"
pub mod wifi;

#[cfg(all(feature = "alloc", feature = "use_serde"))]
pub mod edge_config;
