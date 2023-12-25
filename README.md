## TODO
[![rust-clippy analyze](https://github.com/kriyip/https-proxy-tunnel/actions/workflows/rust-clippy.yml/badge.svg)](https://github.com/kriyip/https-proxy-tunnel/actions/workflows/rust-clippy.yml)
[![Rust](https://github.com/kriyip/https-proxy-tunnel/actions/workflows/rust.yml/badge.svg)](https://github.com/kriyip/https-proxy-tunnel/actions/workflows/rust.yml)

do everything for just TCP first, can implement HTTP and TLS/HTTPS later (and maybe also UDP)
- [x] cached DNS resolver
- [ ] tunnel struct + function implementations
    - [ ] receive client queries, resolve DNS, connect to destination
    - tcp tunneling: accept tcp connections, resolve destination ip with dns, establish new tcp connection to destination server
- [ ] http
- [ ] tls https