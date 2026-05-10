<!-- snippet:skip -->

```gleam title="Gleam"
import kreuzberg

// Note: the Gleam binding does not expose a Gleam-implementable plugin
// trait surface, so plugin unit tests (in the Rust sense — constructing
// a `MyPostProcessor`, calling `process(&mut result, &config).await`,
// asserting on the mutated `ExtractionResult`) must live in the Rust
// crate that defines the plugin.
//
// From Gleam (e.g. with gleeunit), test plugin behaviour at the
// integration boundary: drive `kreuzberg.extract_file_sync` /
// `kreuzberg.extract_bytes_sync` against fixtures and assert on the
// resulting `ExtractionResult`. Use `list_post_processors`,
// `list_validators`, and `list_ocr_backends` to assert that the expected
// plugins are registered before the run.
pub fn main() {
  let assert Ok(processors) = kreuzberg.list_post_processors()
  let assert Ok(validators) = kreuzberg.list_validators()
  let _ = #(processors, validators)
  Nil
}
```
