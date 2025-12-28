# Vector Database Integration
# This example demonstrates how to prepare document chunks for integration with vector databases
# by configuring chunking and processing the extracted content.

alias Kreuzberg.ExtractionConfig

# Configure extraction with chunking enabled
config = %ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "max_chars" => 512,
    "max_overlap" => 50
  }
}

# Extract file with chunking
{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

# Prepare chunks for vector database ingestion
documents = Enum.map(result.chunks || [], fn chunk ->
  %{
    text: chunk["text"],
    metadata: %{
      page: chunk["page"],
      char_count: String.length(chunk["text"])
    }
  }
end)

IO.puts("Prepared #{length(documents)} documents for vector DB")

# The documents list can now be sent to your vector database
# Example: documents |> MyVectorDB.index_documents()
