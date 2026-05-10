<!-- snippet:skip -->

```gleam title="Gleam"
import kreuzberg

// Note: the Gleam binding does not expose per-plugin removal helpers
// (`unregister_post_processor`, `unregister_validator`,
// `unregister_ocr_backend`). The kreuzberg Rust core's removal APIs
// operate on `Arc<RwLock<Registry<dyn Trait>>>` values that the Rustler
// NIF bridge does not surface to Gleam.
//
// To wipe registry state from Gleam, use the bulk `clear_*` helpers
// (`kreuzberg.clear_post_processors`, `kreuzberg.clear_validators`,
// `kreuzberg.clear_ocr_backends`) instead. For targeted removal, do it
// from the Elixir/Rustler side that hosts the kreuzberg NIF.
pub fn main() {
  let _ = kreuzberg.clear_post_processors()
  let _ = kreuzberg.clear_validators()
  let _ = kreuzberg.clear_ocr_backends()
  Nil
}
```
