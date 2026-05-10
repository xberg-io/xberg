```gleam title="Gleam"
import gleam/int
import gleam/io
import gleam/list
import gleam/option
import kreuzberg

fn embedding_config() -> kreuzberg.EmbeddingConfig {
  kreuzberg.EmbeddingConfig(
    model: kreuzberg.Preset(name: "balanced"),
    normalize: True,
    batch_size: 32,
    show_download_progress: False,
    cache_dir: option.None,
    acceleration: option.None,
    max_embed_duration_secs: option.None,
  )
}

pub fn main() {
  let texts = ["Hello, world!", "Kreuzberg is fast"]
  case kreuzberg.embed_texts(texts, embedding_config()) {
    Ok(embeddings) -> {
      io.println("Vectors: " <> int.to_string(list.length(embeddings)))
      case list.first(embeddings) {
        Ok(first) ->
          io.println("Dimensions: " <> int.to_string(list.length(first)))
        Error(_) -> Nil
      }
    }
    Error(_) -> io.println_error("embedding failed")
  }
}
```
