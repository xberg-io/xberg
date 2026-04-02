```ruby title="Ruby"
require "kreuzberg"

config = { model: { type: "preset", name: "balanced" }, normalize: true }
texts = ["Hello, world!", "Kreuzberg is fast"]

# Synchronous
embeddings = Kreuzberg.embed_sync(texts: texts, config: config)
puts embeddings.length    # 2
puts embeddings[0].length # 768

# Async variant (uses same thread, returns when done)
embeddings = Kreuzberg.embed(texts: texts, config: config)
puts embeddings[0].length # 768
```
