```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Build comprehensive configuration for all features
config = %ExtractionConfig{
  # Enable OCR for scanned documents
  ocr: %{
    "enabled" => true,
    "backend" => "tesseract",
    "language" => "eng",
    "force_all_pages" => false
  },
  # Configure chunking for RAG applications
  chunking: %{
    "enabled" => true,
    "max_chars" => 1000,
    "max_overlap" => 100
  },
  # Extract images from documents
  images: %{
    "extract" => true
  },
  # Enable language detection
  language_detection: %{
    "enabled" => true
  },
  # Use caching for performance
  use_cache: true
}

# Extract file with full configuration
case Kreuzberg.extract_file("document.pdf", nil, config) do
  {:ok, result} ->
    IO.puts("=== Extraction Successful ===\n")

    # 1. Process content
    IO.puts("--- Content ---")
    content_preview = String.slice(result.content, 0..100)
    IO.puts("Preview: #{content_preview}...")
    IO.puts("Total size: #{byte_size(result.content)} bytes\n")

    # 2. Process metadata
    IO.puts("--- Metadata ---")
    IO.puts("MIME type: #{result.mime_type}")
    metadata = result.metadata || %{}
    IO.puts("Metadata keys: #{inspect(Map.keys(metadata))}")

    # Handle PDF-specific metadata
    case metadata["pdf"] do
      pdf_meta when is_map(pdf_meta) ->
        IO.puts("  Pages: #{pdf_meta["page_count"]}")
        IO.puts("  Author: #{pdf_meta["author"]}")
        IO.puts("  Title: #{pdf_meta["title"]}")
      _ -> nil
    end
    IO.puts("")

    # 3. Process tables
    IO.puts("--- Tables ---")
    tables = result.tables || []
    IO.puts("Total tables: #{length(tables)}")
    Enum.with_index(tables, 1) |> Enum.each(fn {table, idx} ->
      cells = table["cells"] || []
      IO.puts("  Table #{idx}: #{length(cells)} rows")
      markdown = table["markdown"]
      if markdown, do: IO.puts("    Markdown: #{String.slice(markdown, 0..50)}...")
    end)
    IO.puts("")

    # 4. Process chunks for RAG
    IO.puts("--- Chunks ---")
    chunks = result.chunks || []
    IO.puts("Total chunks: #{length(chunks)}")
    Enum.with_index(chunks, 1) |> Enum.each(fn {chunk, idx} ->
      IO.puts("  Chunk #{idx}: #{byte_size(chunk)} bytes")
    end)
    IO.puts("")

    # 5. Process detected languages
    IO.puts("--- Language Detection ---")
    languages = result.detected_languages || []
    if Enum.empty?(languages) do
      IO.puts("No languages detected")
    else
      Enum.each(languages, fn lang ->
        IO.puts("  Language: #{lang}")
      end)
    end
    IO.puts("")

    # 6. Process images
    IO.puts("--- Images ---")
    images = result.images || []
    IO.puts("Total images: #{length(images)}")
    Enum.with_index(images, 1) |> Enum.each(fn {image, idx} ->
      IO.puts("  Image #{idx}: #{image["format"]} (#{image["size"]} bytes)")
    end)

  {:error, reason} ->
    IO.puts("Extraction failed!")
    IO.puts("Error: #{inspect(reason)}")
end
```
