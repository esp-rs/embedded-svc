// Uncomment once domain 0.6.2 which has no_std support is released
//#[cfg(feature = "experimental")]
//pub mod captive;
#[cfg(feature = "experimental")]
pub mod asynch;
#[cfg(feature = "experimental")]
pub mod asyncify;
#[cfg(all(feature = "experimental", target_has_atomic = "8"))]
pub mod forever;
#[cfg(all(
    feature = "experimental",
    any(feature = "json_io", feature = "json_io_core"),
    feature = "use_serde"
))]
pub mod ghota;
#[cfg(feature = "experimental")]
pub mod http;
pub mod io;
#[cfg(all(
    feature = "experimental",
    any(feature = "json_io", feature = "json_io_core")
))]
pub mod json_io;
pub mod mutex;
#[cfg(all(
    feature = "experimental",
    any(feature = "json_io", feature = "json_io_core"),
    feature = "use_serde"
))]
pub mod rest;
pub mod role;
