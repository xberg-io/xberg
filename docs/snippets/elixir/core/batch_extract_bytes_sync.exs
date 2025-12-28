```elixir title="Elixir"
# Extract from multiple binary inputs in a batch operation
# Useful for processing multiple documents in memory

# Prepare binary data from multiple sources
{:ok, pdf_data_1} = File.read("document1.pdf")
{:ok, pdf_data_2} = File.read("document2.pdf")
{:ok, pdf_data_3} = File.read("document3.pdf")

data_list = [pdf_data_1, pdf_data_2, pdf_data_3]

# Option 1: Use single MIME type for all inputs
{:ok, results} = Kreuzberg.batch_extract_bytes(data_list, "application/pdf")

# Process results
Enum.each(results, fn result ->
  IO.puts("Content length: #{byte_size(result.content)} characters")
  IO.puts("MIME type: #{result.mime_type}")
  IO.puts("Tables found: #{length(result.tables)}")
  IO.puts("---")
end)

IO.puts("Total documents processed: #{length(results)}")

# Option 2: Use different MIME types for each input
mime_types = ["application/pdf", "text/html", "application/pdf"]
{:ok, mixed_results} = Kreuzberg.batch_extract_bytes(data_list, mime_types)

# Option 3: Batch extraction with configuration
config = %Kreuzberg.ExtractionConfig{
  ocr: %{"enabled" => true, "backend" => "tesseract"},
  extract_images: true
}

case Kreuzberg.batch_extract_bytes(data_list, "application/pdf", config) do
  {:ok, results} ->
    IO.puts("Successfully extracted #{length(results)} documents")
    Enum.each(results, fn result ->
      IO.puts("Content: #{String.slice(result.content, 0..100)}...")
    end)

  {:error, reason} ->
    IO.puts("Batch extraction failed: #{reason}")
end

# Option 4: Using the bang variant (raises on error)
try do
  results = Kreuzberg.batch_extract_bytes!(data_list, "application/pdf")
  IO.puts("Extracted #{length(results)} documents successfully")
rescue
  error in Kreuzberg.Error ->
    IO.puts("Error: #{error.message}")
end
```
