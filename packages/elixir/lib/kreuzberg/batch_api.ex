defmodule Kreuzberg.BatchAPI do
  @moduledoc """
  Batch extraction operations for processing multiple documents efficiently.

  This module provides functions for extracting content from multiple files or
  binary inputs in batch operations, which can be more efficient than processing
  files individually when dealing with large numbers of documents.
  """

  alias Kreuzberg.{Error, ExtractionConfig, ExtractionResult, Helpers, Native}

  @doc """
  Extract content from multiple files in a batch operation.

  ## Parameters

    * `paths` - List of file paths (strings or Path.t())
    * `mime_type` - MIME type for all files (optional, defaults to nil for auto-detection)
    * `config` - ExtractionConfig struct or map with extraction options (optional)

  ## Returns

    * `{:ok, results}` - List of ExtractionResult structs
    * `{:error, reason}` - Error message if batch extraction fails

  ## Examples

      # Extract multiple PDFs
      paths = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]
      {:ok, results} = Kreuzberg.BatchAPI.batch_extract_files(paths, "application/pdf")

      # Extract with config
      config = %Kreuzberg.ExtractionConfig{images: %{"enabled" => true}}
      {:ok, results} = Kreuzberg.BatchAPI.batch_extract_files(paths, "application/pdf", config)

      # Auto-detect MIME types
      {:ok, results} = Kreuzberg.BatchAPI.batch_extract_files(paths)
  """
  @spec batch_extract_files(
          [String.t() | Path.t()],
          String.t() | nil,
          ExtractionConfig.t() | map() | keyword() | nil
        ) :: {:ok, [ExtractionResult.t()]} | {:error, String.t()}
  def batch_extract_files(paths, mime_type \\ nil, config \\ nil)
      when is_list(paths) and (is_nil(mime_type) or is_binary(mime_type)) do
    # Convert all paths to strings
    string_paths = Enum.map(paths, &to_string/1)

    case call_native_batch_files(string_paths, mime_type, config) do
      {:ok, results_list} when is_list(results_list) ->
        process_batch_results(results_list, string_paths, "file")

      {:error, _reason} = err ->
        err
    end
  end

  @doc """
  Extract content from multiple files, raising on error.

  Same as `batch_extract_files/3` but raises a `Kreuzberg.Error` exception if extraction fails.

  ## Examples

      paths = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]
      results = Kreuzberg.BatchAPI.batch_extract_files!(paths, "application/pdf")
  """
  @spec batch_extract_files!(
          [String.t() | Path.t()],
          String.t() | nil,
          ExtractionConfig.t() | map() | keyword() | nil
        ) :: [ExtractionResult.t()]
  def batch_extract_files!(paths, mime_type \\ nil, config \\ nil) do
    case batch_extract_files(paths, mime_type, config) do
      {:ok, results} -> results
      {:error, reason} -> raise Error, message: reason, reason: Kreuzberg.UtilityAPI.classify_error(reason)
    end
  end

  @doc """
  Extract content from multiple binary inputs in a batch operation.

  ## Parameters

    * `data_list` - List of binary data inputs
    * `mime_types` - List of MIME types (one per input) or single MIME type for all
    * `config` - ExtractionConfig struct or map with extraction options (optional)

  ## Returns

    * `{:ok, results}` - List of ExtractionResult structs
    * `{:error, reason}` - Error message if batch extraction fails

  ## Examples

      # Extract multiple PDFs from binary data
      data_list = [pdf_binary1, pdf_binary2, pdf_binary3]
      mime_types = ["application/pdf", "application/pdf", "application/pdf"]
      {:ok, results} = Kreuzberg.BatchAPI.batch_extract_bytes(data_list, mime_types)

      # Use single MIME type for all inputs
      {:ok, results} = Kreuzberg.BatchAPI.batch_extract_bytes(data_list, "application/pdf")

      # With config
      config = %Kreuzberg.ExtractionConfig{ocr: %{"enabled" => true}}
      {:ok, results} = Kreuzberg.BatchAPI.batch_extract_bytes(data_list, mime_types, config)
  """
  @spec batch_extract_bytes(
          [binary()],
          String.t() | [String.t()],
          ExtractionConfig.t() | map() | keyword() | nil
        ) :: {:ok, [ExtractionResult.t()]} | {:error, String.t()}
  def batch_extract_bytes(data_list, mime_types, config \\ nil)
      when is_list(data_list) and (is_binary(mime_types) or is_list(mime_types)) do
    # Normalize mime_types to a list
    normalized_mime_types = normalize_mime_types(mime_types, data_list)

    # Validate that we have the same number of inputs and MIME types
    if length(data_list) != length(normalized_mime_types) do
      mismatch_error(data_list, normalized_mime_types)
    else
      case call_native_batch_bytes(data_list, normalized_mime_types, config) do
        {:ok, results_list} when is_list(results_list) ->
          process_batch_results(results_list, normalized_mime_types, "mime_type")

        {:error, _reason} = err ->
          err
      end
    end
  end

  @doc """
  Extract content from multiple binary inputs, raising on error.

  Same as `batch_extract_bytes/3` but raises a `Kreuzberg.Error` exception if extraction fails.

  ## Examples

      data_list = [pdf_binary1, pdf_binary2, pdf_binary3]
      results = Kreuzberg.BatchAPI.batch_extract_bytes!(data_list, "application/pdf")
  """
  @spec batch_extract_bytes!(
          [binary()],
          String.t() | [String.t()],
          ExtractionConfig.t() | map() | keyword() | nil
        ) :: [ExtractionResult.t()]
  def batch_extract_bytes!(data_list, mime_types, config \\ nil) do
    case batch_extract_bytes(data_list, mime_types, config) do
      {:ok, results} -> results
      {:error, reason} -> raise Error, message: reason, reason: Kreuzberg.UtilityAPI.classify_error(reason)
    end
  end

  # Private

  defp normalize_mime_types(mime_types, data_list) do
    if is_binary(mime_types) do
      List.duplicate(mime_types, length(data_list))
    else
      mime_types
    end
  end

  defp mismatch_error(data_list, mime_types) do
    {:error,
     "Mismatch between data_list length (#{length(data_list)}) and mime_types length (#{length(mime_types)})"}
  end

  defp process_batch_results(results_list, reference_list, reference_type) do
    results =
      results_list
      |> Enum.with_index()
      |> Enum.map(fn {result_map, index} ->
        case Helpers.into_result(result_map) do
          {:ok, result} -> {:ok, result}
          {:error, reason} -> {:error, index, reason}
        end
      end)

    # Check if any failed
    case Enum.find(results, fn r -> match?({:error, _, _}, r) end) do
      nil ->
        # All succeeded
        {:ok, Enum.map(results, fn {:ok, result} -> result end)}

      {:error, index, reason} ->
        reference = Enum.at(reference_list, index, "unknown")
        {:error, "Failed at index #{index} (#{reference_type}: '#{reference}'): #{reason}"}
    end
  end

  defp call_native_batch_files(paths, mime_type, config) do
    Helpers.call_native(
      fn -> Native.batch_extract_files(paths, mime_type) end,
      fn config_map -> Native.batch_extract_files_with_options(paths, mime_type, config_map) end,
      config
    )
  end

  defp call_native_batch_bytes(data_list, mime_types, config) do
    Helpers.call_native(
      fn -> Native.batch_extract_bytes(data_list, mime_types) end,
      fn config_map -> Native.batch_extract_bytes_with_options(data_list, mime_types, config_map) end,
      config
    )
  end
end
