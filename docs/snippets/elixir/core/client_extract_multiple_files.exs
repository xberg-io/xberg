```elixir title="Elixir"
# Client wrapper for batch file extraction
# Provides error handling, logging, and result aggregation

defmodule BatchDocumentClient do
  @moduledoc """
  Client wrapper for batch file document extraction.
  Handles multiple files with comprehensive error handling and logging.
  """

  alias Kreuzberg.ExtractionResult

  @doc """
  Extract content from multiple files in batch.

  Returns results for all successfully processed files and logs errors
  for any files that fail during extraction.

  ## Options

    * `:mime_type` - MIME type for all files (optional, defaults to auto-detection)
    * `:config` - ExtractionConfig struct with options (optional)
    * `:log_errors` - Whether to log extraction errors (default: true)
    * `:fail_fast` - Stop on first error (default: false)

  ## Examples

      {:ok, results} = BatchDocumentClient.extract_files(
        ["doc1.pdf", "doc2.pdf", "doc3.pdf"],
        mime_type: "application/pdf"
      )
  """
  @spec extract_files([String.t()], keyword()) ::
          {:ok, [ExtractionResult.t()]} | {:error, String.t()}
  def extract_files(paths, opts \\ []) do
    mime_type = Keyword.get(opts, :mime_type, nil)
    config = Keyword.get(opts, :config, nil)
    log_errors = Keyword.get(opts, :log_errors, true)

    case Kreuzberg.batch_extract_files(paths, mime_type, config) do
      {:ok, results} ->
        IO.debug("Successfully extracted #{length(results)} files")
        {:ok, results}

      {:error, reason} ->
        if log_errors do
          IO.debug("Batch extraction error: #{reason}")
        end
        {:error, reason}
    end
  end

  @doc """
  Extract files and return detailed statistics.

  Returns aggregated metrics about all processed files.
  """
  @spec extract_files_with_stats([String.t()], keyword()) ::
          {:ok, map()} | {:error, String.t()}
  def extract_files_with_stats(paths, opts \\ []) do
    start_time = System.monotonic_time(:millisecond)

    case extract_files(paths, opts) do
      {:ok, results} ->
        elapsed_ms = System.monotonic_time(:millisecond) - start_time

        stats = %{
          total_files: length(results),
          total_content_size: Enum.reduce(results, 0, &(byte_size(&1.content) + &2)),
          total_tables: Enum.reduce(results, 0, &(length(&1.tables) + &2)),
          total_images: Enum.reduce(results, 0, &(length(&1.images || []) + &2)),
          processing_time_ms: elapsed_ms,
          avg_time_per_file_ms: div(elapsed_ms, max(length(results), 1)),
          results: results
        }

        {:ok, stats}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Extract files and transform results.

  Applies a transformation function to each extraction result.
  Useful for custom processing or formatting of results.
  """
  @spec extract_and_transform([String.t()], function(), keyword()) ::
          {:ok, [any()]} | {:error, String.t()}
  def extract_and_transform(paths, transform_fn, opts \\ []) do
    case extract_files(paths, opts) do
      {:ok, results} ->
        transformed =
          results
          |> Enum.map(fn result ->
            try do
              {:ok, transform_fn.(result)}
            rescue
              error ->
                IO.debug("Transform error: #{inspect(error)}")
                {:error, error}
            end
          end)

        # Check if any transforms failed
        case Enum.find(transformed, fn r -> match?({:error, _}, r) end) do
          nil ->
            # All succeeded
            {:ok, Enum.map(transformed, fn {:ok, value} -> value end)}

          {:error, error} ->
            {:error, "Transform failed: #{inspect(error)}"}
        end

      {:error, reason} ->
        {:error, reason}
    end
  end
end

# Usage examples

# Extract multiple files
case BatchDocumentClient.extract_files(["doc1.pdf", "doc2.pdf", "doc3.pdf"]) do
  {:ok, results} ->
    Enum.each(results, fn result ->
      IO.puts("Extracted: #{byte_size(result.content)} bytes")
    end)

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end

# Extract with statistics
case BatchDocumentClient.extract_files_with_stats(["doc1.pdf", "doc2.pdf"]) do
  {:ok, stats} ->
    IO.puts("Total files: #{stats.total_files}")
    IO.puts("Total size: #{stats.total_content_size} bytes")
    IO.puts("Processing time: #{stats.processing_time_ms}ms")

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end

# Extract and transform
transform = fn result ->
  %{
    mime: result.mime_type,
    size: byte_size(result.content),
    tables: length(result.tables)
  }
end

case BatchDocumentClient.extract_and_transform(["doc1.pdf", "doc2.pdf"], transform) do
  {:ok, transformed_results} ->
    IO.inspect(transformed_results)

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end
```
