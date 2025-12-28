# Configure chunking for RAG/vector search
config = %Kreuzberg.ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "max_chars" => 512,
    "max_overlap" => 50,
    "respect_boundaries" => true
  }
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

# Prepare chunks for vector embedding and search
chunks_for_embedding = result.chunks
  |> Enum.map(fn chunk ->
    %{
      "id" => chunk["id"],
      "text" => chunk["text"],
      "metadata" => %{
        "page" => chunk["page"],
        "source" => "document.pdf"
      }
    }
  end)

IO.inspect(chunks_for_embedding, label: "Chunks Ready for RAG")
