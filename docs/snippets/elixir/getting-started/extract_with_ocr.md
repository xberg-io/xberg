```elixir title="Elixir"
defmodule ExtractWithOcr do
  def extract_scanned_document do
    # Build configuration with OCR settings as JSON string
    config = ~s({"ocr": {"backend": "tesseract", "language": "eng"}})

    case Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "scanned.pdf"}, config) do
      {:ok, output} ->
        result = List.first(output.results)
        IO.puts("Extracted via OCR:")
        IO.puts(result.content)
        :ok

      {:error, reason} ->
        IO.puts("OCR extraction failed: #{reason}")
        :error
    end
  end
end
```
