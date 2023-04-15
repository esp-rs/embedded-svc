#![cfg_attr(not(feature = "std"), no_std)]
#![feature(cfg_version)]
#![cfg_attr(
    all(feature = "nightly", not(version("1.65"))),
    feature(generic_associated_types)
)]
#![cfg_attr(feature = "nightly", feature(type_alias_impl_trait))]
#![cfg_attr(
    all(feature = "nightly", version("1.70")),
    feature(impl_trait_in_assoc_type)
)]
#![cfg_attr(feature = "nightly", feature(async_fn_in_trait))]
#![cfg_attr(feature = "nightly", allow(incomplete_features))]

#[cfg(feature = "alloc")]
#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

#[cfg(all(feature = "defmt", feature = "log"))]
compile_error!("You must enable at most one of the following features: defmt, log");

pub mod eth;
pub mod event_bus;
pub mod executor;
#[cfg(feature = "experimental")]
pub mod http;
#[cfg(feature = "std")]
#[deprecated(since = "0.22.0", note = "Use module http::server")]
pub mod httpd; // TODO: Retire
pub mod io;
pub mod ipv4;
pub mod macros;
pub mod mqtt;
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
