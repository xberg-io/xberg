#ifndef RUST_BRIDGE_C_H
#define RUST_BRIDGE_C_H

// Placeholder header for the RustBridgeC SwiftPM target.
// Run `cargo build -p kreuzberg-swift` and re-run `alef all` to populate.
// The RustStr typedef below is the minimum required for SwiftBridgeCore.swift
// to compile before the full cargo build has been run.

#include <stdint.h>

typedef struct RustStr { uint8_t* const start; uintptr_t len; } RustStr;

#endif /* RUST_BRIDGE_C_H */
