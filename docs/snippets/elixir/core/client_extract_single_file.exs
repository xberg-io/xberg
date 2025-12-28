```elixir title="Elixir"
# Reusable client pattern for single file extraction
# Encapsulates extraction logic with consistent error handling

defmodule DocumentClient do
  @moduledoc """
  Client wrapper for single file document extraction.
  Provides a consistent interface for extracting content from files.
  """

  alias Kreuzberg.ExtractionResult

  @doc """
  Extract content from a single file.

  Returns a result tuple with the extraction outcome.
  Supports explicit MIME type specification or auto-detection.

  ## Options

    * `:mime_type` - MIME type of the file (optional, defaults to auto-detection)
    * `:config` - ExtractionConfig struct with options (optional)

  ## Examples

      {:ok, result} = DocumentClient.extract_file("document.pdf")
      {:ok, result} = DocumentClient.extract_file("document.pdf", mime_type: "application/pdf")
  """
  @spec extract_file(String.t(), keyword()) ::
          {:ok, ExtractionResult.t()} | {:error, String.t()}
  def extract_file(path, opts \\ []) do
    mime_type = Keyword.get(opts, :mime_type, nil)
    config = Keyword.get(opts, :config, nil)

    case Kreuzberg.extract_file(path, mime_type, config) do
      {:ok, result} ->
        IO.debug("Successfully extracted file: #{path}")
        {:ok, result}

      {:error, reason} ->
        IO.debug("Failed to extract file: #{path} - #{reason}")
        {:error, reason}
    end
  end

  @doc """
  Extract content from a file, raising on error.

  Raises Kreuzberg.Error if extraction fails.
  """
  @spec extract_file!(String.t(), keyword()) :: ExtractionResult.t()
  def extract_file!(path, opts \\ []) do
    mime_type = Keyword.get(opts, :mime_type, nil)
    config = Keyword.get(opts, :config, nil)

    Kreuzberg.extract_file!(path, mime_type, config)
  end

  @doc """
  Extract and process file content.

  Returns a map with extracted content, metadata, and processing statistics.
  """
  @spec extract_with_stats(String.t(), keyword()) ::
          {:ok, map()} | {:error, String.t()}
  def extract_with_stats(path, opts \\ []) do
    start_time = System.monotonic_time(:millisecond)

    case extract_file(path, opts) do
      {:ok, result} ->
        elapsed_ms = System.monotonic_time(:millisecond) - start_time

        {:ok,
         %{
           content: result.content,
           mime_type: result.mime_type,
           metadata: result.metadata,
           table_count: length(result.tables),
           image_count: length(result.images || []),
           processing_time_ms: elapsed_ms
         }}

      {:error, reason} ->
        {:error, reason}
    end
  end
end

# Usage examples
case DocumentClient.extract_file("document.pdf") do
  {:ok, result} ->
    IO.puts("Content length: #{byte_size(result.content)} bytes")

  {:error, reason} ->
    IO.puts("Extraction failed: #{reason}")
end

# Extract with statistics
case DocumentClient.extract_with_stats("document.pdf") do
  {:ok, stats} ->
    IO.puts("Processing time: #{stats.processing_time_ms}ms")
    IO.puts("Tables found: #{stats.table_count}")

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end
```
