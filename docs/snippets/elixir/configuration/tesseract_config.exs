```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Configure Tesseract OCR settings
# Includes language selection, page segmentation mode, OCR engine mode, and DPI settings
config = %ExtractionConfig{
  ocr: %{
    "enabled" => true,
    "backend" => "tesseract",
    "language" => "eng",
    "psm" => 3,
    "oem" => 3,
    "dpi" => 300
  },
  use_cache: true,
  force_ocr: false
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

IO.puts("Tesseract Configuration Applied:")
IO.puts("Backend: tesseract")
IO.puts("Language: eng")
IO.puts("PSM (Page Segmentation Mode): 3")
IO.puts("OEM (OCR Engine Mode): 3")
IO.puts("DPI: 300")
IO.puts("Content extracted: #{byte_size(result.content)} bytes")
IO.puts("Metadata: #{inspect(result.metadata)}")
```
