<!-- snippet:syntax-only -->

```elixir
config_json =
  Jason.encode!(%{
    "chunking" => %{
      "max_characters" => 500,
      "overlap" => 50,
      "embedding" => %{
        "model" => %{"preset" => %{"name" => "balanced"}},
        "normalize" => true,
        "batch_size" => 16
      }
    }
  })

{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "research_paper.pdf"}, config_json)

result = List.first(output.results)
chunks_with_embeddings =
  for chunk <- result.chunks || [],
      embedding = chunk["embedding"],
      is_list(embedding) do
    %{
      content: String.slice(chunk["content"] || "", 0, 100),
      embedding_dims: length(embedding)
    }
  end

IO.puts("Chunks with embeddings: #{length(chunks_with_embeddings)}")
```
