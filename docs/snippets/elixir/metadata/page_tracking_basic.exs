```elixir title="Elixir"
# Track which pages content originated from during extraction
# Useful for cross-referencing extracted content back to source documents

alias Kreuzberg.ExtractionConfig

config = %ExtractionConfig{
  # Standard extraction configuration
  use_cache: true
}

{:ok, result} = Kreuzberg.extract_file("multi_page_document.pdf", nil, config)

# Access metadata to retrieve page information
metadata = result.metadata || %{}

# For PDF documents, metadata includes page tracking
case metadata["pdf"] do
  pdf_meta when is_map(pdf_meta) ->
    IO.puts("Total pages in document: #{pdf_meta["page_count"]}")
    IO.puts("Document title: #{pdf_meta["title"]}")
    IO.puts("Document author: #{pdf_meta["author"]}")
  _ ->
    IO.puts("No PDF metadata available")
end

# When using chunks, track which content came from which page
chunks = result.chunks || []
IO.puts("\nTotal chunks extracted: #{length(chunks)}")

# Process chunks and associate with page information
Enum.with_index(chunks, 1) |> Enum.each(fn {chunk, index} ->
  # Estimate page number based on chunk position
  # This is a simplified approach - actual implementation may vary
  text = Map.get(chunk, "text", "")
  IO.puts("Chunk #{index}: #{byte_size(text)} bytes")
  IO.puts("  Content preview: #{String.slice(text, 0..80)}...")
end)

# For tables, metadata may include page numbers
tables = result.tables || []
IO.puts("\nTotal tables found: #{length(tables)}")

Enum.with_index(tables, 1) |> Enum.each(fn {table, idx} ->
  cells = table["cells"] || []
  IO.puts("Table #{idx}: #{length(cells)} rows")

  # Table metadata may indicate source page
  case table["metadata"] do
    meta when is_map(meta) ->
      IO.puts("  Page: #{meta["page"] || "Unknown"}")
    _ ->
      IO.puts("  Page: Unknown")
  end
end)

# Track images with page information
images = result.images || []
IO.puts("\nTotal images found: #{length(images)}")

Enum.with_index(images, 1) |> Enum.each(fn {image, idx} ->
  IO.puts("Image #{idx}:")
  IO.puts("  Format: #{image["format"]}")
  IO.puts("  Size: #{image["size"]} bytes")

  # Page information if available in image metadata
  if image["page"] do
    IO.puts("  Page: #{image["page"]}")
  end
end)
```
