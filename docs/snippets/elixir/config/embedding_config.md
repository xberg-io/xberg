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

input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
chunks_count = if result.chunks, do: length(result.chunks), else: 0
IO.puts("Chunks with embeddings: #{chunks_count}")
```
