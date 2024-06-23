# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.28.0] - 2024-06-23
### Breaking
* Add configuration for Protected Management Frames and scan methods to `wifi::ClientConfiguration`
* Removed the `no-std-net` dependency in favor of `core::net` which is stable since Rust 1.77
* Due to the above, module `ipv4` no longer re-exports `ToSocketAddrs` with `feature = "std"` enabled as this trait is not available in `core::net` (and might completely stop re-exporting `core::net` types in future)

## [0.27.1] - 2024-02-21
* Fix clippy duplicate imports warnings with latest 1.78 nightly

## [0.27.0] - 2024-01-26
* MAJOR CLEANUP/REMOVAL of obscure/rarely used trait modules:
  * `sys_time`
  * `ping`
  * `timer` (as there is `embedded_hal::delay` and more importantly - `embedded_hal_async::delay`)
  * `event_bus`
  * `ws::callback_server`
* COMPLETE REMOVAL of the following utility modules:
  * `asyncify` - all functionality which still made sense merged into `esp-idf-svc`
  * `utils::mutex` - this was a helper module for `asyncify`, so removed completely
* New module: `channel`; similar to crate `channel-bridge` but with less constraints on the data being sent/received
* Breaking change in modules `mqtt::client` and `utils::mqtt::client`: The `Event` structure, and its associated `Message` and `MessageImpl` traits simplified significantly, allowing for much more ergonomic event processing, thanks to GATs and async-fn-in-trait which are now stable:
  * Introduced a new single-method trait: `Event` with method `payload` returning `EventPayload`
  * Introduced a new enumeration - `EventPayload` - modeling all possible event types that can be received from the MQTT client
  * `Message` and `MessageImpl` retired
* Breaking change in module `http::server`: `HandlerError` and `HandlerResult` are now gone. 
  * The blocking and async versions of the `Handler` and `Middleware` traits now all have an associated `Error` type that the user can define however she wants (only requirement is for it to implement `Debug`)
  * Additionally, the blocking and async versions of `Middleware` are now generified by the `Handler` type, so that they have access
    to the Handler's error type and are therefore free to implement their error type in terms of a composition between the handler error type and the error types of other functions they are calling
* Bumped the MSRV version to 1.75 and removed the `nightly` feature requirement from all async traits
* The `serde` dependency is now optional
* Update to `heapless` 0.8

## [0.26.4] - 2023-11-12
* Updated changelog

## [0.26.3] - 2023-11-12
* BREAKING CHANGE IN A PATCH RELEASE DUE TO DISCOVERED UB: Traits `EventBus` and `TimerService` no longer allow subscriptions with non-static callbacks, 
as these are impossible to implement safely on ESP IDF

## [0.26.2] - 2023-11-05
* A temporary workaround for https://github.com/rust-lang/rust/issues/117602

## [0.26.1] - 2023-10-18
* Rolled back a change where `event_bus::asynch::Sender` and `event_bus::asynch::Receiver` did no longer implement `ErrorType` and returned a `Result`; since these traits are rarely used (feature `nightly` only), and 0.26.0 was just released, no new major version was released, but instead 0.26.0 was yanked

## [0.26.0] - 2023-10-17
* MSRV raised to 1.71
* Breaking change: All traits converted to AFIT, except `Unblocker`, which needs to compile with stable Rust
* Breaking change: Upgraded to `embedded-io` 0.5 and `embedded-io-async` 0.5
* Upgraded `strum` and `strum-macros` to 0.25
* OTA: New method: `Ota::finish` that allows to postpone/avoid setting the updated partition as a boot one
* TimerService: `TimerService::timer` now takes `&self` instead of `&mut self` - for both blocking and async traits
* Breaking change: TimerService: scoped handler: the timer callback now only needs to live as long as the `TimerService::Timer` associated type. Therefore, `TimerService::Timer` is now lifetimed: `TimerService::Timer<'a>`
* Breaking change: TimerService: `TimerService::Timer` now borrows from `TimerService`. Therefore, that's another reason why `TimerService::Timer` is now lifetimed: `TimerService::Timer<'a>`
* Breaking change: EventBus: scoped handler: the subscription callback now only needs to live as long as the `EventBus::Subscription` associated type. Therefore, `EventBus::Subscription` is now lifetimed: `EventBus::Subscription<'a>`
* Breaking change: EventBus: `EventBus::Subscription` now borrows from `EventBus`. Therefore, that's another reason why `EventBus::Subscription` is now lifetimed: `EventBus::Subscription<'a>`
* Breaking change: ws::Acceptor: the blocking as well as the async version now return `Connection` / `Sender` / `Receiver` instances which borrow from `Acceptor`
* Breaking change: Unblocker: scoped handler: the callback now only needs to live as long as the `Unblocker::UnblockFuture` associated type. Therefore, `Unblocker::UnblockFuture` is now lifetimed: `Unblocker::UnblockFuture<'a, ...>`
* Breaking change: Unblocker: `Unblocker::UnblockFuture` now borrows from `Unblocker::UnblockFuture`. Therefore, that's another reason why `Unblocker::UnblockFuture` is now lifetimed: `Unblocker::UnblockFuture<'a, ...>`
* Breaking change: OTA: GAT `Ota::Update` now parametric over lifetime and no longer returned by `&mut` ref
* Breaking change: OTA: `OtaUpdate::abort` and `OtaUpdate::complete` now take `self` instead of `&mut self`
* Breaking change: MQTT: GAT `Connection::Message` now parametric over lifetime
* Breaking change: Ping: Callback function of `Ping::ping_details` can now be `FnMut` but does require `Send`
* Breaking change: All pub structs in `utils::asyncify` that implement the `Future` trait are now private and wrapped with async methods
* Breaking change: Removed structs `Blocking` and `TrivialAsync`, as well as all trait implementations on them, because their usefulness was questionable
* Breaking change: Removed the deprecated module `httpd` and the dependency on `anyhow`

## [0.25.3] - 2023-07-05
* Compatibility with latest Rust nightly Clippy (fixes the "usage of `Arc<T>` where `T` is not `Send` or `Sync`" error)

## [0.25.2] - 2023-07-05
* Yanked; first attempt at ^^^

## [0.25.1] - 2023-06-18
* Compatibility with latest Rust nightly (fixes the `can't leak private types` error)

## [0.25.0] - 2023-05-13

* MSRV 1.66 (but MSRV 1.70 necessary if `embedded-io-async` is enabled)
* Remove the `experimental` status from all traits
* Remove the `nightly` feature flag guard from all `asyncify` utilities as Rust GATs are stable now
* Async `Wifi` and `Eth` traits
* `defmt` support
* Mask the SSID password in Wifi `ClientConfiguration`
* Switch from `futures` to the `atomic_waker` crate in the `asyncify` utilities
* Minor breaking change: `is_up` renamed to `is_connected` in the `Eth` trait
* Upgrade to `embedded-io` 0.4 (but still stay away from switching the async traits to the `async` syntax)

## [0.24.0] - 2022-12-13

HTTP server traits:
* Change the signatures of `Handler`, `Middleware`, `asynch::Handler` and `asynch::Middleware` so that middleware implementations can use the HTTP connection after the handler has finished execution
* Remove the `handler` utility method, as it was adding little value besides calling `FnHandler::new()`
* Remove the `FnConnectionHandler` Fn `Handler` implementation, as it was confusing to have it in addition to the `FnHandler` implementation

## [0.23.2] - 2022-12-08

* Const functions for strum_enums
* Defmt support

## [0.23.1] - 2022-11-21

Patch release to fix compilation errors under no_std.

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

This module underwent complete refactoring. Major changes:
* `Connection` trait: this is the major HTTP abstraction for both client code and server handlers. Amongst other reasons, introducing this trait solves the problem where the underlying TCP socket abstraction cannot be split into separate "reader" and "writer". Note that certain methods of the `Connection` trait can only be called when the connection is in a certain state (i.e. "client request submitted" phase vs "client request submitted" phase) and will panic otherwise. There are two safe, non-panicking wrappers of `Connection` for both HTTP client and server: `Request` and `Response`, where the recommendation for user code is to use the `Connection` trait via the `Request` wrapper (`Request` is automatically turned into `Response` once the request is submitted)
* On error, the server `Handler` trait is now required to return a dedicated `HandlerError` structure, which is just a wrapper around an error message. `HandlerError` is expected to be turned into an HTTP 500 response by trait implementors. The generified `E: Debug` error that used to be returned by the `Handler` trait introduced a very complex lifetime handling in user code and was therefore retired. Note that since `HandlerError` does not allocate, the maximum error message that it can contain is 64 characters. Longer messages are automatically truncated.
* The `registry` module is removed, as the `Registry` trait was impossible to implement without allocations. Instead, the new `utils::http::registry` utility is offered
* `Query` trait, allowing user to retrive the HTTP method and URI
* Implement traits on `& T` and `&mut T` where appropriate
* `headers` module with utility functions for building a headers' array for submission
* `Blocking` and `TrivialUnblocking` adaptors from async to blocking traits and vice versa
* `Header` utility in `utils::http`
* The `session` module is significantly simplified and moved to `utils::http::session`
* The `cookies` module is moved to `utils::http::cookies`

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

##### utils::mutex

This is a module that provides mutex and condvar abstractions, but with the caveat that these abstractions are supposed to *only* be used by:
* Other utilities in the `utils` module, namely the `utils::asyncify` module, and the helper `utils::mqtt` module (the latter can optionally be used by implementors of the synchronous `mqtt` traits)
* Implementors of the `embedded-svc` traits

It is *not* intended for general, public use. 
If users need a synchronous mutex or condvar abstractions for their application code, they are strongly encouraged to use one of the following options:
* For STD-compatible environments, the mutex and condvar abstractions provided by the Rust STD library itself
* For no_std environments, one of the following:
 * The synchronous mutex abstractions provided by the `embassy-sync` crate
 * The critical section provided by the `critical-section` crate
 * Note that the no_std options above only provide a mutex abstration. If users need a condvar abstraction (usually only the case for RTOS environments which provde a thread/task notion), they should use the native condvar facilities of their RTOS

Furthermore:
 * When the `embedded-svc` utilities are used in a STD-compatible environment, the mutex and condvar abstractions of `utils::mutex` 
are already implemented in terms of Rust's STD mutex and condvar abstractions, and users should not be concerned with this module at all
 * When the `embedded-svc` utilities are used in a no_std environment (i.e., an RTOS that provides blocking synchronization primitives, as well as the notion of task/threads but these are not Rust STD compatible), users are required to provide implementations of the `RawMutex` and `RawCondvar` traits

#### wifi

`TransitionalState` struct is retired. The `Wifi` trait is redesigned to have explicit `start`, `stop`, `connect` and `disconnect` methods, as well as `is_started` and `is_connected` methods that to some extent provide the functionality 
which used to be available via the now-retired `get_status` method.

Furthermore, the `Wifi` trait now no longer assumes that the driver implementing it is capable of operating above the OSI Layer-2. In other words, a driver implementing the `Wifi` trait might, or might not bring up an IP-level stack. As such, all structures related to "IP" are retired or moved to the `ipv4` module.
