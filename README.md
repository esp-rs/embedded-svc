# Rust APIs and abstractions for various embedded services (WiFi, Network, Httpd, Logging, etc.)

The APIs currently have a single implementation for the [ESP32/ESP-IDF](https://github.com/ivmarkov/esp-idf-svc), using the [Xtensa/ESP32 Rust-STD](https://github.com/ivmarkov/rust) compiler fork.
<br><br>
However, they are portable and should be possible to implement for other boards too.

For more information, check out:
* The [Rust Xtensa STD port](https://github.com/ivmarkov/rust)
* The ["Hello, World" demo](https://github.com/ivmarkov/rust-esp32-std-hello)
* The [esp-idf-svc](https://github.com/ivmarkov/esp-idf-svc) project
