<!-- snippet:skip -->

```gleam title="Gleam"
import kreuzberg

// Note: the Gleam binding does not expose `register_document_extractor`.
// Built-in extractors (PDF, DOCX, HTML, etc.) are registered automatically
// by the kreuzberg core when the Rustler NIF loads. Custom extractors
// must be implemented in Rust and linked into a custom build of the
// kreuzberg NIF — Gleam cannot bridge a `dyn DocumentExtractor` trait
// object back through Rustler.
//
// You can confirm which extractors are live with `list_document_extractors`.
pub fn main() {
  let assert Ok(_extractors) = kreuzberg.list_document_extractors()
  Nil
}
```
