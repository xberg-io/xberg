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

{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, config_json)

result = List.first(output.results)
IO.puts("chunks: #{length(result.chunks || [])}")
```
