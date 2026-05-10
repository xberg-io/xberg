<!-- snippet:skip -->

```gleam title="Gleam"
import kreuzberg

// Note: the Gleam binding does not expose a Gleam-implementable
// `Validator` trait. The Rust trait is `Send + Sync + 'static` with an
// async `validate(&ExtractionResult, &ExtractionConfig) -> Result<()>`
// signature; Rustler cannot bridge that to a Gleam callback module
// directly.
//
// `kreuzberg.register_validator` accepts an Erlang PID that must already
// belong to a GenServer answering `{:trait_call, method, args_json,
// reply_id}` messages and replying via the `validator_*_response` shims.
// Per the kreuzberg Gleam docs, that GenServer is wired from the
// Elixir/Rustler side. Write validator logic in Rust and register it
// from the host Rust binary loading the kreuzberg NIF.
pub fn main() {
  Nil
}
```
