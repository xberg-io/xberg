<!-- snippet:skip -->

```gleam title="Gleam"
import kreuzberg

// Note: the Gleam binding does not expose a Gleam-implementable
// `PostProcessor` trait. `kreuzberg.register_post_processor` accepts an
// Erlang `pid` whose owning GenServer must answer
// `{:trait_call, method, args_json, reply_id}` messages via `handle_info/2`
// and reply with the corresponding `post_processor_*_response` shim.
//
// Per the kreuzberg Gleam module docs, "wiring the callback module is
// done via the Elixir/Rustler side (existing GenServer pattern)" — there
// is no idiomatic Gleam-only way to host that GenServer today. Implement
// the post-processor in Rust against `kreuzberg::plugins::PostProcessor`
// (e.g. a word-count processor that fills `Metadata.word_count`) and
// register it from the host Rust binary that loads the kreuzberg NIF.
pub fn main() {
  Nil
}
```
