# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.23] - 2022-11-01

Release 0.23 is a backwards-incompatible release where almost all traits were touched in one way or another.

### Major Changes

The main themes of the 0.23 release are:
* Lose weight - the `utils` module was reduced to the bare minimum necessary. After all, the main goal of this crate is to provide traits, not utilities
* In addition to all traits being implementable in `no_std` environments, make sure they do **not** have explicit or implicit dependencies on an allocator being available (the Rust `alloc` module)
* Improve the experimental HTTP client and server traits
* Separate the notions of using a "nightly" compiler (which is a precondition for all async support) from "experimental" features, which might or might not be async-related

### Changes by module

#### channel

This module is now moved to a separate micro-crate: `channel-bridge`. The traits in `embedded-svc` no longer depend on the generic `Receiver` and `Sender` traits the `channel` module used to provide.

#### errors

This module is completely retired, as it used to provide utilities (`EitherErrorXX`) which were anyway used only by a handful of the traits in `embedded-svc`.
Those traits now use their own custom error types.

#### eth

`TransitionalState` struct is retired. The `Eth` trait is redesigned to have explicit `start` and `stop` methods, as well as `is_started` and `is_up` methods that to some extent provide the functionality 
which used to be available via the now-retired `get_status` method.

Furthermore, the `Eth` trait now no longer assumes that the driver implementing it is capable of operating above the OSI Layer-2. In other words, a driver implementing the `Eth` trait might, or might not bring up an IP-level stack. As such, all structures related to "IP" are retired or moved to the `ipv4` module.

#### event_bus

* All trait methods on the blocking traits now take `&self` instead of `&mut self`. Given that `EventBus` and `Postbox` are essentially a publish-subscribe channel in disguise this makes a lot of sense
* Implement `EventBus` and `Postbox` for `& T` and `&mut T`
* `PinnedEventBus` is now retired, as its usefulness was queztionable (it was only lifting the `Send` requirement on the subscription callback)
* Async traits: remove the dependency on `channel::Receiver` and `channel::Sender`

#### executor

* All existing traits retired, as they cannot be implemented without an allocator being available (i.e. not implementable on top of the `embassy-executor` crate)
* New traits (and utility structs around them):  `Blocker` and `Unblocker`. Note that `Unblocker` is currently not possible to implement without an allocation either, but it was kept for completeness.

#### http

TODO

#### httpd

This module is now marked as deprecated. The successor is the `http::server` module.

#### io

* Blocking and async utility methods like `read_max` and `copy` moved to `utils::io`

#### ipv4

* `Configuration` struct describing the IP-related configuration of network interfaces, along with an accopmanying `Interface` trait

#### mqtt

* `&mut T` traits implementations

#### mutex

* Retired as it used GATs. While GATs are stable in the meantime, this trait still had open questions around it, as in the "guard" pattern it standardizes is not really embraced by the embedded community. Use the raw `BlockingMutex` trait provided by the `embassy-sync` crate along with its ergonomic typed wrapper, or - for STD-only environments - the STD `Mutex` directly.

#### ota

* `OtaServer` retired. This trait had nothing to do with the capabilities of the device itself in terms of OTA support, and modeled a generic "OTA server". While perhaps a useful concept, it might had been too early for that
* `Slot` trait turned into a structure, similar to `Firmware`. This enabled GAT-free blocking traits. While GATs are now getting stabilized, we still try to use those only for async traits, where they are absolute necessity
* Implement `Ota` for `&mut T`

#### ping

* Implement `Ping` for `&mut T`

#### storage

* `StorageImpl` - out of the box `Storage` implementation based on `RawStorage` impl and `SerDe` impl provided by the user

#### sys_time, timer

* `& T` and `&mut T` impls where appropriate

#### unblocker

* Removed, see the `executor` module for a suitable replacement

#### utils

Retired copies of async utilities available in other crates:
* `signal`, `mutex`, `channel` (were copied from the `embassy-sync` crate, which is now published on crates.io)
* `yield_now`, `select` (were copied from the `embassy-futures` crate, which is now published on crates.io)
* `forever` (were copied from the `static_cell` crate, which is now published on crates.io)

Utilities moved to other crates:
* `executor` - available as a separate `edge-executor` micro-crate now
* `ghota` - available as a separate `ghota` micro-crate now
* `captive` - available as part of the `edge-net` crate now
* `role` - available as part of the `edge-frame` crate now

Completely retired utilities:
* `rest`
* `json_io`

TODO: `utils::mutex`

#### wifi

`TransitionalState` struct is retired. The `Wifi` trait is redesigned to have explicit `start`, `stop`, `connect` and `disconnect` methods, as well as `is_started` and `is_connected` methods that to some extent provide the functionality 
which used to be available via the now-retired `get_status` method.

Furthermore, the `Wifi` trait now no longer assumes that the driver implementing it is capable of operating above the OSI Layer-2. In other words, a driver implementing the `Wifi` trait might, or might not bring up an IP-level stack. As such, all structures related to "IP" are retired or moved to the `ipv4` module.
