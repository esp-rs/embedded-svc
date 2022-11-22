# Rust APIs and abstractions for embedded services

[![CI](https://github.com/esp-rs/embedded-svc/actions/workflows/ci.yml/badge.svg)](https://github.com/esp-rs/embedded-svc/actions/workflows/ci.yml)
![crates.io](https://img.shields.io/crates/v/embedded-svc.svg)
[![Documentation](https://docs.rs/embedded-svc/badge.svg)](https://docs.rs/embedded-svc)

This crate ships traits for embedded features such as wifi, networking, httpd, logging.
The intended use is for concrete implementations to use the traits provided in this crate as a common base.
This would eventually lead to a portable embedded ecosystem. The APIs currently have a single implementation for the [ESP32[-XX] / ESP-IDF](https://github.com/esp-rs/esp-idf-svc).
However, they are meant to be portable and should be possible to implement for other boards too.

For more information, check out:
* The [Rust on ESP Book](https://esp-rs.github.io/book/)
* The [esp-idf-svc](https://github.com/esp-rs/esp-idf-svc) project
* The [esp-idf-template](https://github.com/esp-rs/esp-idf-template) project
* The [esp-idf-sys](https://github.com/esp-rs/esp-idf-sys) project
* The [esp-idf-hal](https://github.com/esp-rs/esp-idf-hal) project
* The [Rust for Xtensa toolchain](https://github.com/esp-rs/rust-build)
* The [Rust-with-STD demo](https://github.com/ivmarkov/rust-esp32-std-demo) project
