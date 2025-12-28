# Quality processing configuration
alias Kreuzberg.ExtractionConfig

config = %ExtractionConfig{
  quality_processing: %{
    "enabled" => true,
    "min_quality_score" => 0.75,
    "remove_noise" => true,
    "enhance_clarity" => true
  },
  ocr: %{
    "enabled" => true,
    "backend" => "tesseract"
  }
}

# Extract file with quality processing and OCR
{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

# Process the quality-processed results
IO.puts("=== Quality Processing Results ===\n")

# Display extracted content with quality processing applied
content_preview = String.slice(result.content, 0..300)
IO.puts("Extracted Content (first 300 chars):")
IO.puts(content_preview)
IO.puts("\nTotal content size: #{byte_size(result.content)} bytes")

# Check quality metrics in metadata
quality_score = result.metadata["quality_score"]
if quality_score do
  IO.puts("\nQuality Score: #{quality_score}")
  IO.puts("Quality Status: #{if quality_score >= 0.75, do: "Acceptable", else: "Below threshold"}")
end

# Display any OCR results if applicable
if result.images && length(result.images) > 0 do
  IO.puts("\nImages found: #{length(result.images)}")
end
