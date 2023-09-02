#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(
    feature = "nightly",
    feature(async_fn_in_trait, impl_trait_projections)
)]
#![cfg_attr(feature = "nightly", feature(impl_trait_in_assoc_type))]
#![allow(clippy::unused_unit)] // enumset

#[cfg(feature = "alloc")]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

#[cfg(all(feature = "defmt", feature = "log"))]
compile_error!("You must enable at most one of the following features: defmt, log");

pub mod eth;
pub mod event_bus;
pub mod executor;
pub mod http;
#[cfg(feature = "std")]
#[deprecated(since = "0.22.0", note = "Use module http::server")]
pub mod httpd; // TODO: Retire
pub mod io;
pub mod ipv4;
pub mod macros;
pub mod mqtt;
pub mod ota;
pub mod ping;
pub mod storage;
pub mod sys_time;
pub mod timer;
pub mod utils;
pub mod wifi;
pub mod ws;
