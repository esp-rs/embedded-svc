// Uncomment once domain 0.6.2 which has no_std support is released
//#[cfg(feature = "experimental")]
//pub mod captive;

#[cfg(feature = "experimental")]
pub mod asyncify;

#[cfg(feature = "experimental")]
pub mod asyncs;

#[cfg(all(feature = "experimental", feature = "alloc", feature = "use_serde"))]
pub mod ghota;

#[cfg(all(feature = "experimental", feature = "alloc", feature = "use_serde"))]
pub mod rest;

pub mod role;
