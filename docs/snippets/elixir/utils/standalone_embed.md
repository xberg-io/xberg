```elixir
# Embed with default config
{:ok, embeddings} = Kreuzberg.embed(["Hello world", "How are you?"])

# Embed with specific preset
config = %Kreuzberg.EmbeddingConfig{model: {:preset, "fast"}}
{:ok, embeddings} = Kreuzberg.embed(["Hello world"], config)

# Raise on error
embeddings = Kreuzberg.embed!(["Hello world"])
```
