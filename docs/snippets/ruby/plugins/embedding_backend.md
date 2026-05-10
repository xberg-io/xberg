```ruby title="Ruby"
require 'kreuzberg'

# Wrap an already-loaded embedder so kreuzberg can call back into it during
# chunking and standalone embed requests. The Ruby object must respond to
# `dimensions` and `embed`; `version`, `initialize`, and `shutdown` are
# optional lifecycle hooks.
class MyEmbedder
  def version
    '1.0.0'
  end

  def initialize_plugin
    # Optional warm-up; runs once at registration.
  end

  def shutdown
    # Optional cleanup.
  end

  # Captured once at registration; the dispatcher uses this for shape validation.
  def dimensions
    768
  end

  def embed(texts)
    # Delegate to the already-loaded host model.
    texts.map { Array.new(768, 0.0) }
  end
end

# Register once at startup. The second argument is the plugin name used to
# reference the backend from EmbeddingConfig.
Kreuzberg.register_embedding_backend(MyEmbedder.new, 'my-embedder')

config = Kreuzberg::EmbeddingConfig.new(
  model: { type: 'plugin', name: 'my-embedder' },
  # Optional: bound the wait on a hung backend (default 60s; nil disables).
  max_embed_duration_secs: 30
)

vectors = Kreuzberg.embed_texts(['Hello, world!', 'Second text'], config: config)
puts "Generated #{vectors.length} vectors"
```
