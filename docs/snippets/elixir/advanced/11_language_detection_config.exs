# Advanced Language Detection Configuration
# This example demonstrates how to configure language detection with multiple
# parameters for detecting and tracking all languages in documents.

alias Kreuzberg.ExtractionConfig

# Advanced language detection configuration
config = %ExtractionConfig{
  language_detection: %{
    "enabled" => true,
    "detect_all" => true,
    "min_confidence" => 0.8
  }
}

# Extract file with language detection enabled
{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

# Process the detection results
detection_results = result.detected_languages || []

IO.puts("Detected languages:")

Enum.each(detection_results, fn lang_result ->
  confidence = Map.get(lang_result, "confidence", "unknown")
  language = Map.get(lang_result, "language", "unknown")
  IO.puts("  - #{language}: #{confidence}")
end)

# Filter results by minimum confidence threshold
high_confidence_languages = Enum.filter(detection_results, fn lang_result ->
  confidence = Map.get(lang_result, "confidence", 0)
  confidence >= 0.8
end)

IO.puts("\nHigh confidence languages (>= 0.8): #{length(high_confidence_languages)}")
