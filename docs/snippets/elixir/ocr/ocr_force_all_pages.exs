```elixir title="Elixir"
# Force OCR processing on all pages of a document
# By default, OCR is only applied when needed (scanned content detected)
# Use force_all_pages to ensure OCR runs on every page regardless

alias Kreuzberg.ExtractionConfig

# Configuration with OCR forced on all pages
config = %ExtractionConfig{
  ocr: %{
    "enabled" => true,
    "backend" => "tesseract",
    "language" => "eng",
    # Force OCR to run on every page, not just scanned content
    "force_all_pages" => true
  },
  chunking: %{
    "enabled" => true,
    "max_chars" => 1500,
    "max_overlap" => 150
  },
  language_detection: %{
    "enabled" => true
  },
  use_cache: true
}

IO.puts("Starting OCR extraction with force_all_pages enabled...")
start_time = System.monotonic_time(:millisecond)

case Kreuzberg.extract_file("mixed_document.pdf", nil, config) do
  {:ok, result} ->
    elapsed_ms = System.monotonic_time(:millisecond) - start_time

    IO.puts("\n=== OCR Extraction Complete ===\n")

    # Show timing information
    IO.puts("Processing time: #{elapsed_ms}ms")
    IO.puts("Content extracted: #{byte_size(result.content)} bytes")

    # Metadata shows OCR was performed
    metadata = result.metadata || %{}
    if metadata["ocr_applied"] do
      IO.puts("OCR applied to all pages: Yes")
    end

    # Show language detection results
    languages = result.detected_languages || []
    IO.puts("\nDetected languages (#{length(languages)}):")
    Enum.each(languages, fn lang ->
      IO.puts("  - #{lang}")
    end)

    # Show chunking results (useful for RAG/search)
    chunks = result.chunks || []
    IO.puts("\nChunks created: #{length(chunks)}")
    avg_chunk_size = if Enum.empty?(chunks) do
      0
    else
      total_size = Enum.reduce(chunks, 0, &(byte_size(&1) + &2))
      div(total_size, length(chunks))
    end
    IO.puts("Average chunk size: #{avg_chunk_size} bytes")

    # Display content preview
    IO.puts("\nContent preview (first 300 characters):")
    preview = String.slice(result.content, 0..299)
    IO.puts(preview)
    IO.puts("...\n")

    # Show any extracted tables
    tables = result.tables || []
    if not Enum.empty?(tables) do
      IO.puts("Tables found: #{length(tables)}")
      Enum.with_index(tables, 1) |> Enum.each(fn {table, idx} ->
        cells = table["cells"] || []
        IO.puts("  Table #{idx}: #{length(cells)} rows")
      end)
      IO.puts("")
    end

    # Show any extracted images
    images = result.images || []
    if not Enum.empty?(images) do
      IO.puts("Images extracted: #{length(images)}")
      Enum.with_index(images, 1) |> Enum.each(fn {image, idx} ->
        IO.puts("  Image #{idx}: #{image["format"]} - #{image["size"]} bytes")
      end)
    end

  {:error, reason} ->
    elapsed_ms = System.monotonic_time(:millisecond) - start_time
    IO.puts("OCR extraction failed after #{elapsed_ms}ms")
    IO.puts("Error: #{inspect(reason)}")
end
```
