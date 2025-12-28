```elixir title="Elixir"
alias Kreuzberg.Plugin

# PDF-Only Post-Processor Plugin
# This conditional post-processor only processes PDF files.
# It can be used to apply PDF-specific transformations.

defmodule MyApp.Plugins.PdfOnlyProcessor do
  @behaviour Kreuzberg.Plugin.PostProcessor
  require Logger

  @impl true
  def name do
    "PdfOnlyProcessor"
  end

  @impl true
  def processing_stage do
    :post
  end

  @impl true
  def version do
    "1.0.0"
  end

  @impl true
  def initialize do
    :ok
  end

  @impl true
  def shutdown do
    :ok
  end

  @impl true
  def process(result, _options) do
    mime_type = result["mime_type"] || ""

    if mime_type == "application/pdf" do
      # Process only for PDF files
      Logger.info("Processing PDF content with custom PDF processor")

      # Example: Extract PDF-specific metadata or transform content
      content = result["content"] || ""

      # Add PDF processing marker to metadata
      metadata = Map.get(result, "metadata", %{})
      updated_metadata = Map.put(metadata, "processed_by_pdf_processor", true)

      {:ok, Map.put(result, "metadata", updated_metadata)}
    else
      {:ok, result}
    end
  end
end

# Register the PDF-only post-processor
Plugin.register_post_processor(:pdf_only_processor, MyApp.Plugins.PdfOnlyProcessor)

# Example usage with PDF result
pdf_result = %{
  "content" => "PDF extracted content here",
  "mime_type" => "application/pdf",
  "metadata" => %{
    "source" => "document.pdf",
    "pages" => 5
  }
}

# Process PDF result
case MyApp.Plugins.PdfOnlyProcessor.process(pdf_result, %{}) do
  {:ok, processed_result} ->
    IO.puts("PDF processing complete")
    IO.inspect(processed_result, label: "PDF Result")

  {:error, reason} ->
    IO.puts("PDF processing failed: #{reason}")
end

# Example with non-PDF result (processor will skip processing)
non_pdf_result = %{
  "content" => "Image extracted content",
  "mime_type" => "image/png",
  "metadata" => %{}
}

case MyApp.Plugins.PdfOnlyProcessor.process(non_pdf_result, %{}) do
  {:ok, processed_result} ->
    IO.puts("Processing complete (skipped for non-PDF)")
    IO.inspect(processed_result, label: "Non-PDF Result")

  {:error, reason} ->
    IO.puts("Processing failed: #{reason}")
end
```
