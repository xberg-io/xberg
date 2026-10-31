---
status: accepted
date: 2026-05-15
decision-makers: ["Na'aman Hirschfeld"]
---

# Java on Panama FFM, JNI only for Kotlin Android

## Context and Problem Statement

The JVM had three routes into the Rust core. The Java package bound the C FFI crate through Panama's Foreign
Function & Memory API since 03c0c0399f ("feat(java): add Panama FFI bindings for Java 22+", 2025-11-14). A separate
Kotlin/JVM package, `packages/kotlin`, depended on JNA (`net.java.dev.jna:jna`) and wrapped the alef-emitted Java
facade. An Android AAR, `packages/kotlin-android`, loaded `kreuzberg_ffi` and called it through Kotlin `external fun`
declarations; there was no `-jni` crate yet. alef v0.16.0 split Kotlin into a dedicated `kotlin_android` backend
that emits a self-contained AAR project and no longer emitted `packages/kotlin`. alef generates every binding, as
[org ADR-0002 (Rust cores with alef-generated bindings)](https://github.com/xberg-io/agent-conventions/blob/main/adrs/0002-rust-cores-with-alef-generated-bindings.md)
decides.

## Considered Options

* Java through Panama FFM over `xberg-ffi`, server-side Kotlin through the Java binding, and a JNI shim only for the
  Kotlin Android AAR
* A separate Kotlin/JVM package over JNA beside the Java and Android packages

## Decision Outcome

Chosen option: "Java through Panama FFM over `xberg-ffi`, server-side Kotlin through the Java binding, and a JNI shim
only for the Kotlin Android AAR", because 6b18cf612a (2026-05-15) retired `packages/kotlin` on the grounds that
"Server-side Kotlin/JVM consumers now use the Java binding directly (kotlin-java interop is seamless), and the
Android distribution lives under packages/kotlin-android/".

* `alef.toml` `[crates.java]` sets `ffi_style = "panama"`: the Java package calls `xberg-ffi`, the same C ABI that
  Go, C#, Zig and Swift use, and binds exactly `[crates.ffi]`'s feature set.
* The JNI half settled after this decision. e31cba78d5 (2026-05-30) added the `-jni` shim crate for JNI symbol
  resolution, and cc3b153131 (2026-06-12) declared `[crates.jni]` for it, "Paired with kotlin_android backend".
  `crates/xberg-jni` exists only for `packages/kotlin-android`.
* The Java package requires Java 25 with `--enable-preview` and `--enable-native-access=ALL-UNNAMED`.

### Consequences

* Good, because desktop and server JVM users share one package, with no JNA or JNI layer between Java and the C ABI.
* Good, because the Java binding rides on the C FFI crate's ABI, so an FFI fix reaches Java with the other C-ABI
  languages.
* Bad, because the Java package requires Java 25 and preview flags.
* Bad, because the Android AAR is a second JVM-family binding with its own JNI crate, feature list and ABI matrix
  (`arm64-v8a`, `x86_64`).

### Confirmation

* `alef.toml:2386` sets `ffi_style = "panama"` under `[crates.java]` (`:2384`). `[crates.kotlin_android]` (`:2626`)
  and `[crates.jni]` (`:2722`) configure the Android path.
* `packages/java/pom.xml:53` sets `maven.compiler.release` to 25.
* The crate-structure skill states that the Java package does not go through `xberg-jni`
  (`.ai-rulez/skills/crate-structure/SKILL.md:21`).
* No `packages/kotlin` directory exists at origin/main.
