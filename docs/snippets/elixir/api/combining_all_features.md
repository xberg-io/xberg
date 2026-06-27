```elixir title="Elixir"
defmodule Example do
  def full_extraction_pipeline do
    # Build a comprehensive extraction config as a JSON string or map
    config_json = Jason.encode!(%{
      "use_cache" => true,
      "enable_quality_processing" => true,
      "force_ocr" => false,
      "ocr" => %{
        "backend" => "tesseract",
        "language" => "eng"
      },
      "chunking" => %{
        "max_characters" => 800,
        "overlap" => 100,
        "chunker_type" => "Markdown",
        "prepend_heading_context" => true
      },
      "output_format" => "Markdown",
      "include_document_structure" => true,
      "images" => %{
        "extract_images" => true
      },
      "language_detection" => %{
        "detect" => true
      }
    })

    case Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "report.pdf"}, config_json) do
      {:ok, output} ->
        result = List.first(output.results)
        IO.puts("Extraction successful")
        IO.puts("Content length: #{String.length(result.content || "")} chars")
        :ok

      {:error, reason} ->
        IO.puts("Extraction failed: #{reason}")
        :error
    end
  end
end
```
