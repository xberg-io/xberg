# Configure chunking for RAG/vector search
config = %Xberg.ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "max_characters" => 512,
    "overlap" => 50,
    "respect_boundaries" => true
  }
}

{:ok, result} = Xberg.extract("document.pdf", nil, config)

# Prepare chunks for vector embedding and search
chunks_for_embedding = result.chunks
  |> Enum.map(fn chunk ->
    %{
      "id" => chunk["id"],
      "content" => chunk["content"],
      "metadata" => %{
        "page" => chunk["page"],
        "source" => "document.pdf"
      }
    }
  end)

IO.inspect(chunks_for_embedding, label: "Chunks Ready for RAG")
