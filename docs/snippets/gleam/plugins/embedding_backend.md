<!-- snippet:skip -->

```gleam title="Gleam"
import kreuzberg

// Note: the Gleam binding does not expose a Gleam-implementable
// `EmbeddingBackend` trait. `kreuzberg.register_embedding_backend` takes
// an Erlang `pid` for a GenServer that answers `{:trait_call, "embed",
// args_json, reply_id}` and `{:trait_call, "dimensions", ...}` messages
// and replies via `embedding_backend_embed_response` /
// `embedding_backend_dimensions_response` — that GenServer is wired on
// the Elixir/Rustler side per the kreuzberg Gleam module docs.
//
// For pure Gleam usage, prefer the built-in embedding presets — list
// them with `kreuzberg.list_embedding_presets`, fetch one with
// `kreuzberg.get_embedding_preset`, and embed via
// `kreuzberg.embed_texts` / `kreuzberg.embed_texts_async`.
pub fn main() {
  let _presets = kreuzberg.list_embedding_presets()
  Nil
}
```
