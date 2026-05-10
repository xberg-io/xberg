<!-- snippet:skip -->

```gleam title="Gleam"
import kreuzberg

// Note: the Gleam binding does not expose a way to implement a custom
// `DocumentExtractor` from Gleam. The Rust trait carries `Send + Sync +
// 'static` bounds and an async `extract_*` API that Rustler cannot
// bridge to a BEAM-side callback module.
//
// Write the extractor in Rust against `kreuzberg::plugins::DocumentExtractor`
// and register it via `get_document_extractor_registry().write()?.register(...)`
// in the host Rust binary that loads the kreuzberg NIF, then drive it from
// Gleam via the standard `kreuzberg.extract_*` entry points.
pub fn main() {
  let assert Ok(_extractors) = kreuzberg.list_document_extractors()
  Nil
}
```
