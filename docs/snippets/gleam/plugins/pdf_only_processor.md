<!-- snippet:skip -->

```gleam title="Gleam"
import kreuzberg

// Note: the Gleam binding does not expose a Gleam-implementable
// `PostProcessor` trait. `kreuzberg.register_post_processor` requires a
// callback GenServer PID that handles `{:trait_call, method, args_json,
// reply_id}` messages including `should_process` (which is what a
// PDF-only post-processor would gate on `mime_type == "application/pdf"`).
//
// That GenServer must be implemented on the Elixir/Rustler side per the
// existing trait-bridge pattern. Implement the PDF-only post-processor
// in Rust against `kreuzberg::plugins::PostProcessor` and gate it on
// `should_process(&ExtractionResult, ...) -> bool` instead.
pub fn main() {
  Nil
}
```
