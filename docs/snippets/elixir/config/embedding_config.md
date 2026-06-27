```elixir title="Elixir"
config_json = Jason.encode!(%{
  "chunking" => %{
    "max_characters" => 1000,
    "overlap" => 200,
    "embedding" => %{
      "model" => %{
        "preset" => %{
          "name" => "balanced"
        }
      },
      "batch_size" => 16,
      "normalize" => true,
      "show_download_progress" => true
    }
  }
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)
chunks_count = if result.chunks, do: length(result.chunks), else: 0
IO.puts("Chunks with embeddings: #{chunks_count}")
```
