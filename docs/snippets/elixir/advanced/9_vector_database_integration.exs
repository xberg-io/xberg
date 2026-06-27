# Vector Database Integration
# This example demonstrates how to prepare document chunks for integration with vector databases
# by configuring chunking and processing the extracted content.

alias Xberg.ExtractionConfig

# Configure extraction with chunking enabled
config = %ExtractionConfig{
  chunking: %{
    "enabled" => true,
    "max_characters" => 512,
    "overlap" => 50
  }
}

# Extract file with chunking
{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, config)
result = List.first(output.results)
# Prepare chunks for vector database ingestion
documents = Enum.map(result.chunks || [], fn chunk ->
  %{
    content: chunk["content"],
    metadata: %{
      page: chunk["page"],
      char_count: String.length(chunk["content"])
    }
  }
end)

IO.puts("Prepared #{length(documents)} documents for vector DB")

# The documents list can now be sent to your vector database
# Example: documents |> MyVectorDB.index_documents()
