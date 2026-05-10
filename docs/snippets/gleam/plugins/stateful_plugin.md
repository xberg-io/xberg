<!-- snippet:skip -->

```gleam title="Gleam"
import kreuzberg

// Note: the Gleam binding does not expose a Gleam-implementable plugin
// trait surface (`PostProcessor`, `Validator`, `OcrBackend`,
// `EmbeddingBackend`). A stateful plugin — one that holds a counter,
// cache, or other shared mutable state across calls — therefore cannot
// be authored in Gleam and registered through Rustler.
//
// Write the stateful plugin in Rust using `Arc<Mutex<...>>` or
// `AtomicUsize` for shared state, register it from the host Rust binary
// that loads the kreuzberg NIF, and drive it from Gleam via the standard
// `kreuzberg.extract_*` entry points. Inspect the live registry from
// Gleam with `list_post_processors` / `list_validators` /
// `list_ocr_backends`.
pub fn main() {
  let assert Ok(_processors) = kreuzberg.list_post_processors()
  Nil
}
```
