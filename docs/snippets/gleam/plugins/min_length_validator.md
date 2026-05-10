<!-- snippet:skip -->

```gleam title="Gleam"
import kreuzberg

// Note: the Gleam binding does not expose a Gleam-implementable
// `Validator` trait. `kreuzberg.register_validator` takes an Erlang `pid`
// for a GenServer that must answer `{:trait_call, "validate", args_json,
// reply_id}` messages and reply via `validator_validate_response` — the
// "wiring the callback module is done via the Elixir/Rustler side" caveat
// from the kreuzberg Gleam docs applies here.
//
// Implement the minimum-length validator in Rust against
// `kreuzberg::plugins::Validator`, returning `KreuzbergError::validation`
// when `result.content.len() < min_length`, and register it from the host
// Rust binary that loads the kreuzberg NIF.
pub fn main() {
  Nil
}
```
