```elixir title="Elixir"
config_json = Jason.encode!(%{
  "chunking" => %{
    "enabled" => true,
    "max_characters" => 512,
    "overlap" => 50
  },
  "embeddings" => %{
    "enabled" => true,
    "model" => "all-MiniLM-L6-v2"
  }
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)

# Prepare data for vector database storage
vector_db_records = result.chunks
  |> Enum.with_index(1)
  |> Enum.map(fn {chunk, idx} ->
    %{
      "id" => "#{result.document_id}-chunk-#{idx}",
      "vector" => chunk["embedding"],
      "metadata" => %{
        "content" => chunk["content"],
        "page" => chunk["page"],
        "document_id" => result.document_id,
        "chunk_index" => idx
      }
    }
  end)

IO.puts("Generated #{length(vector_db_records)} records for vector database")
IO.inspect(List.first(vector_db_records), label: "Sample Record")

# Example: Insert into Pinecone-like vector database
Enum.each(vector_db_records, fn record ->
  # vector_db_client.upsert(record)
  IO.puts("Would insert: #{record["id"]}")
end)
```
