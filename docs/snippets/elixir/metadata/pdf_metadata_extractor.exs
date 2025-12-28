```elixir title="Elixir"
# Extract and process comprehensive PDF metadata
# Useful for document indexing, cataloging, and metadata-driven workflows

alias Kreuzberg.ExtractionConfig

defmodule PDFMetadataExtractor do
  @moduledoc """
  Extract and process PDF metadata from documents.
  Provides structured access to PDF properties and document information.
  """

  @doc """
  Extract PDF metadata from a file.

  Returns a map with normalized metadata fields.
  """
  def extract_metadata(file_path) do
    config = %ExtractionConfig{
      use_cache: true
    }

    case Kreuzberg.extract_file(file_path, nil, config) do
      {:ok, result} ->
        process_metadata(result.metadata || %{})

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Process and normalize PDF metadata.
  """
  defp process_metadata(metadata) do
    case metadata["pdf"] do
      pdf_meta when is_map(pdf_meta) ->
        {:ok,
         %{
           page_count: pdf_meta["page_count"] || 0,
           title: pdf_meta["title"],
           author: pdf_meta["author"],
           subject: pdf_meta["subject"],
           keywords: pdf_meta["keywords"],
           creator: pdf_meta["creator"],
           producer: pdf_meta["producer"],
           creation_date: pdf_meta["creation_date"],
           modification_date: pdf_meta["modification_date"],
           version: pdf_meta["version"],
           is_encrypted: pdf_meta["is_encrypted"] || false,
           is_tagged: pdf_meta["is_tagged"] || false
         }}

      _ ->
        {:error, "No PDF metadata found"}
    end
  end

  @doc """
  Format metadata for display.
  """
  def format_metadata(metadata) when is_map(metadata) do
    """
    === PDF Metadata ===
    Title: #{metadata[:title] || "N/A"}
    Author: #{metadata[:author] || "N/A"}
    Subject: #{metadata[:subject] || "N/A"}
    Keywords: #{inspect(metadata[:keywords]) || "N/A"}
    Creator: #{metadata[:creator] || "N/A"}
    Producer: #{metadata[:producer] || "N/A"}

    === Document Properties ===
    Pages: #{metadata[:page_count]}
    Version: #{metadata[:version] || "N/A"}
    Encrypted: #{metadata[:is_encrypted]}
    Tagged (Accessible): #{metadata[:is_tagged]}

    === Dates ===
    Created: #{metadata[:creation_date] || "N/A"}
    Modified: #{metadata[:modification_date] || "N/A"}
    """
  end
end

# Usage example
file_path = "document.pdf"

case PDFMetadataExtractor.extract_metadata(file_path) do
  {:ok, metadata} ->
    IO.puts(PDFMetadataExtractor.format_metadata(metadata))

    # Perform metadata-driven operations
    if metadata[:page_count] > 100 do
      IO.puts("Note: Document is large (#{metadata[:page_count]} pages)")
    end

    if metadata[:is_encrypted] do
      IO.puts("Note: Document is password-protected")
    end

    if metadata[:is_tagged] do
      IO.puts("Note: Document is accessible with tags")
    end

  {:error, reason} ->
    IO.puts("Error extracting metadata: #{reason}")
end
```
