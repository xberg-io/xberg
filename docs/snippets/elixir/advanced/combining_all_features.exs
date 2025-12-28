# Use all major features together
config = %Kreuzberg.ExtractionConfig{
  ocr: %{"enabled" => true},
  chunking: %{"enabled" => true, "max_chars" => 1000},
  images: %{"extract" => true},
  language_detection: %{"enabled" => true},
  keyword_extraction: %{"enabled" => true}
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

# Process results with all extracted features
IO.inspect(result, label: "Extraction Result")

# Access different feature outputs
if result.content, do: IO.puts("Text: #{String.slice(result.content, 0..100)}")
if result.detected_languages, do: IO.puts("Language: #{inspect(result.detected_languages)}")
if result.metadata["keywords"], do: IO.puts("Keywords: #{inspect(result.metadata["keywords"])}")
if result.chunks, do: IO.puts("Chunks: #{length(result.chunks)}")
if result.images, do: IO.puts("Images: #{length(result.images)}")
