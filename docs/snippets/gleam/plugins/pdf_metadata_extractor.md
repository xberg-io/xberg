<!-- snippet:skip -->

```gleam title="Gleam"
import kreuzberg

// Note: the Gleam binding does not expose custom `DocumentExtractor`
// registration. A PDF metadata extractor that pulls custom fields out of
// a PDF and emits them on `Metadata` must be written in Rust against the
// `kreuzberg::plugins::DocumentExtractor` trait and compiled into the
// kreuzberg NIF — Rustler cannot bridge a Gleam-implemented async trait
// object back into the Rust registry.
//
// From Gleam, you read the resulting metadata from `ExtractionResult.metadata`
// after calling `kreuzberg.extract_file_sync` / `kreuzberg.extract_bytes_sync`.
pub fn main() {
  Nil
}
```
