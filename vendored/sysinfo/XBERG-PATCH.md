# sysinfo 0.39.6 iOS and compiler compatibility patch

Upstream source: [sysinfo v0.39.6](https://github.com/GuillaumeGomez/sysinfo/tree/1bc6ed4f7d0dd03606de572a1531a165438b63ec), commit `1bc6ed4f7d0dd03606de572a1531a165438b63ec`.
The package source is copied byte-for-byte from backport commit `df81bf56cdd0d2a212726bc72b9e7e0a65dcd91b`.
The original standalone `Cargo.toml` is retained rather than Cargo's normalized registry manifest.

`xberg-ios-network.patch` is the exact diff from that upstream commit to the backport; its SHA-256 is `3c9aa5c2dbf0ca2363fea1c5b4a52235c9d3b60a08cf4efbb68325ee79929f4f`.
It changes only `src/lib.rs`, `src/unix/mod.rs`, `src/unix/apple/mod.rs`, `src/unix/linux/process.rs`, and `tests/network.rs`.
The backport uses the existing unsupported-network implementation on iOS, where the network APIs used by 0.39 are unavailable, and preserves the 0.39 API and macOS implementation.

Two Linux/Android atomic updates use `try_update`, stabilized in Rust 1.95, instead of its deprecated `fetch_update` alias.
This preserves the declared Rust 1.95 minimum and lets path-patched builds retain `-D warnings` on Rust 1.99 without suppressions.

The copy includes 142 upstream package source files, including the MIT license, documentation includes, examples, and tests.
Generated package metadata and lock files, Git ignore metadata, and upstream CI configuration are omitted.

Remove this directory and the Cargo override once the Hugging Face/Xet dependency chain accepts a published sysinfo version with the iOS fix.
