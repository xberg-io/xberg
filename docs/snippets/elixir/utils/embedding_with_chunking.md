<!-- snippet:syntax-only -->

```elixir
config_json =
  Jason.encode!(%{
    "chunking" => %{
      "max_characters" => 1024,
      "overlap" => 100,
      "embedding" => %{
        "model" => %{"preset" => %{"name" => "balanced"}},
        "normalize" => true,
        "batch_size" => 32,
        "show_download_progress" => false
      }
    }
  })

{:ok, json} = Xberg.extract_async("document.pdf", nil, config_json)
result = Jason.decode!(json)
IO.puts("chunks: #{length(result["chunks"] || [])}")
```
