#![cfg_attr(not(feature = "std"), no_std)]

pub mod eth;
#[cfg(feature = "alloc")] // TODO: Expose a subset which does not require "alloc"
pub mod wifi;
pub mod ipv4;
pub mod ping;
#[cfg(feature = "std")] // TODO: Lower requirements to "alloc"
pub mod httpd;
#[cfg(feature = "alloc")] // TODO: Expose a subset which does not require "alloc"
pub mod storage;
#[cfg(feature = "alloc")]
pub mod anyerror;

#[cfg(all(feature = "alloc", feature = "use_serde"))]
pub mod edge_config;
