<!-- snippet:skip -->

```gleam title="Gleam"
import kreuzberg

// Note: the Gleam binding does not expose a Gleam-implementable plugin
// trait, so there is no Gleam-side `process` / `validate` body in which
// to emit structured `tracing` events the way the Rust plugin examples
// do. Plugin-internal logging happens inside the Rust core that owns
// the `dyn PostProcessor` / `dyn Validator` implementation.
//
// From Gleam, log around the kreuzberg call boundary instead — for
// example with `gleam/io` or your application logger — and use
// `list_post_processors` / `list_validators` / `list_ocr_backends` to
// record which plugins are active for a given extraction run.
pub fn main() {
  let assert Ok(processors) = kreuzberg.list_post_processors()
  let assert Ok(validators) = kreuzberg.list_validators()
  let _ = #(processors, validators)
  Nil
}
```
