```elixir title="Elixir"
# Client wrapper with comprehensive configuration
# Demonstrates advanced extraction patterns with OCR, chunking, and validation

defmodule ConfiguredDocumentClient do
  @moduledoc """
  Client wrapper for document extraction with advanced configuration.
  Supports OCR, chunking, language detection, and custom error handling.
  """

  alias Kreuzberg.{ExtractionConfig, ExtractionResult}

  @doc """
  Extract with OCR enabled for scanned documents.

  Uses Tesseract OCR backend for text extraction from images.
  """
  @spec extract_with_ocr(String.t() | binary(), keyword()) ::
          {:ok, ExtractionResult.t()} | {:error, String.t()}
  def extract_with_ocr(input, opts \\ []) do
    is_file = is_binary(input) and File.exists?(input)

    config = %ExtractionConfig{
      ocr: %{
        "enabled" => true,
        "backend" => Keyword.get(opts, :ocr_backend, "tesseract")
      },
      force_ocr: Keyword.get(opts, :force_ocr, false)
    }

    mime_type = Keyword.get(opts, :mime_type, nil)

    case is_file do
      true -> Kreuzberg.extract_file(input, mime_type, config)
      false -> Kreuzberg.extract(input, mime_type || "application/pdf", config)
    end
  end

  @doc """
  Extract with text chunking for embedding or RAG pipelines.

  Splits extracted text into chunks with configurable size and overlap.
  """
  @spec extract_with_chunking(String.t(), keyword()) ::
          {:ok, map()} | {:error, String.t()}
  def extract_with_chunking(path, opts \\ []) do
    config = %ExtractionConfig{
      chunking: %{
        "max_chars" => Keyword.get(opts, :chunk_size, 1000),
        "max_overlap" => Keyword.get(opts, :chunk_overlap, 100)
      }
    }

    mime_type = Keyword.get(opts, :mime_type, nil)

    case Kreuzberg.extract_file(path, mime_type, config) do
      {:ok, result} ->
        chunks = result.chunks || []

        {:ok,
         %{
           content: result.content,
           chunks: chunks,
           chunk_count: length(chunks),
           metadata: result.metadata
         }}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Extract with language detection and multi-language support.

  Detects document languages and can extract from specific languages.
  """
  @spec extract_with_language_detection(String.t(), keyword()) ::
          {:ok, map()} | {:error, String.t()}
  def extract_with_language_detection(path, opts \\ []) do
    config = %ExtractionConfig{
      language_detection: %{"enabled" => true},
      extract_images: true
    }

    mime_type = Keyword.get(opts, :mime_type, nil)

    case Kreuzberg.extract_file(path, mime_type, config) do
      {:ok, result} ->
        {:ok,
         %{
           content: result.content,
           detected_languages: result.detected_languages || [],
           mime_type: result.mime_type,
           tables: result.tables,
           images: result.images || []
         }}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Extract with comprehensive configuration for production use.

  Combines OCR, chunking, caching, and language detection with error handling.
  """
  @spec extract_with_full_config(String.t(), keyword()) ::
          {:ok, map()} | {:error, String.t()}
  def extract_with_full_config(path, opts \\ []) do
    config = %ExtractionConfig{
      # OCR settings
      ocr: %{
        "enabled" => Keyword.get(opts, :ocr_enabled, true),
        "backend" => Keyword.get(opts, :ocr_backend, "tesseract")
      },
      # Chunking for embeddings
      chunking: %{
        "max_chars" => Keyword.get(opts, :chunk_size, 1000),
        "max_overlap" => Keyword.get(opts, :chunk_overlap, 100)
      },
      # Language detection
      language_detection: %{"enabled" => Keyword.get(opts, :detect_language, true)},
      # Cache results
      use_cache: Keyword.get(opts, :use_cache, true),
      # Extract various content types
      extract_images: Keyword.get(opts, :extract_images, true),
      extract_tables: true
    }

    mime_type = Keyword.get(opts, :mime_type, nil)

    case Kreuzberg.extract_file(path, mime_type, config) do
      {:ok, result} ->
        summary = %{
          file_path: path,
          mime_type: result.mime_type,
          content_length: byte_size(result.content),
          content_preview: String.slice(result.content, 0..200),
          detected_languages: result.detected_languages || [],
          table_count: length(result.tables),
          image_count: length(result.images || []),
          chunk_count: length(result.chunks || []),
          metadata: result.metadata
        }

        {:ok, summary}

      {:error, reason} ->
        {:error, "Extraction failed: #{reason}"}
    end
  end

  @doc """
  Validate file before extraction.

  Checks file existence and MIME type compatibility.
  """
  @spec validate_file(String.t()) :: :ok | {:error, String.t()}
  def validate_file(path) do
    cond do
      not File.exists?(path) ->
        {:error, "File not found: #{path}"}

      true ->
        case Kreuzberg.detect_mime_type_from_path(path) do
          {:ok, _mime_type} ->
            :ok

          {:error, reason} ->
            {:error, "Cannot determine MIME type: #{reason}"}
        end
    end
  end

  @doc """
  Extract with validation and error recovery.

  Validates file before extraction and provides detailed error information.
  """
  @spec extract_safely(String.t(), keyword()) ::
          {:ok, map()} | {:error, String.t()}
  def extract_safely(path, opts \\ []) do
    with :ok <- validate_file(path),
         {:ok, summary} <- extract_with_full_config(path, opts) do
      {:ok, summary}
    else
      {:error, reason} ->
        {:error, reason}
    end
  end
end

# Usage examples

# Extract with OCR
case ConfiguredDocumentClient.extract_with_ocr("scanned_document.pdf",
  ocr_backend: "tesseract"
) do
  {:ok, result} ->
    IO.puts("OCR extraction successful")
    IO.puts("Content: #{String.slice(result.content, 0..100)}...")

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end

# Extract with chunking
case ConfiguredDocumentClient.extract_with_chunking("document.pdf",
  chunk_size: 500,
  chunk_overlap: 50
) do
  {:ok, data} ->
    IO.puts("Chunks: #{data.chunk_count}")

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end

# Extract with language detection
case ConfiguredDocumentClient.extract_with_language_detection("multilingual.pdf") do
  {:ok, data} ->
    IO.puts("Detected languages: #{inspect(data.detected_languages)}")

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end

# Comprehensive extraction with validation
case ConfiguredDocumentClient.extract_safely("document.pdf",
  ocr_enabled: true,
  detect_language: true,
  extract_images: true,
  use_cache: true
) do
  {:ok, summary} ->
    IO.puts("File: #{summary.file_path}")
    IO.puts("MIME: #{summary.mime_type}")
    IO.puts("Size: #{summary.content_length} bytes")
    IO.puts("Tables: #{summary.table_count}")
    IO.puts("Languages: #{inspect(summary.detected_languages)}")

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end
```
