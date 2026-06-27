```elixir title="Elixir"
defmodule ExtractWithOcr do
  def extract_scanned_document do
    # Build configuration with OCR settings as JSON string
    config = ~s({"ocr": {"backend": "tesseract", "language": "eng"}})

    case Xberg.extract_sync("scanned.pdf", nil, config) do
      {:ok, result} ->
        IO.puts("Extracted via OCR:")
        IO.puts(result)
        :ok

      {:error, reason} ->
        IO.puts("OCR extraction failed: #{reason}")
        :error
    end
  end
end
```
