# Extract and prepare for embeddings
config = %Kreuzberg.ExtractionConfig{
  chunking: %{"enabled" => true, "max_chars" => 512},
  embeddings: %{"enabled" => true}
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

# Process chunks with embeddings for semantic search
embedded_chunks = result.chunks
  |> Enum.with_index(1)
  |> Enum.map(fn {chunk, idx} ->
    %{
      "chunk_id" => idx,
      "text" => chunk["text"],
      "embedding" => chunk["embedding"],
      "page" => chunk["page"],
      "metadata" => %{
        "document" => "document.pdf",
        "chunk_index" => idx
      }
    }
  end)

# Store embeddings in vector database
IO.puts("Prepared #{length(embedded_chunks)} chunks with embeddings")
IO.inspect(embedded_chunks, label: "Embedded Chunks")
