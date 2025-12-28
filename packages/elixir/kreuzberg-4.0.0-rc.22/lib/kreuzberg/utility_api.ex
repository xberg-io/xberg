defmodule Kreuzberg.UtilityAPI do
  @moduledoc """
  Utility functions for Kreuzberg extraction operations.

  This module provides helper functions for MIME type detection and validation,
  extension mapping, embedding preset management, and error classification.
  These utilities are essential for pre-extraction validation and post-extraction
  analysis.

  ## MIME Type Operations

  - `detect_mime_type/1` - Detect MIME type from binary data using content inspection
  - `detect_mime_type_from_path/1` - Detect MIME type from file path using extension
  - `validate_mime_type/1` - Validate that a MIME type string is supported
  - `get_extensions_for_mime/1` - Get file extensions associated with a MIME type

  ## Embedding Presets

  - `list_embedding_presets/0` - List all available embedding model presets
  - `get_embedding_preset/1` - Get detailed information about a specific preset

  ## Error Handling

  - `classify_error/1` - Classify error messages into semantic error categories
  - `get_error_details/0` - Get information about all error categories

  ## Examples

      # MIME type detection
      {:ok, mime_type} = Kreuzberg.UtilityAPI.detect_mime_type(pdf_binary)
      {:ok, mime_type} = Kreuzberg.UtilityAPI.detect_mime_type_from_path("document.pdf")

      # MIME type validation
      {:ok, _} = Kreuzberg.UtilityAPI.validate_mime_type("application/pdf")
      {:error, _} = Kreuzberg.UtilityAPI.validate_mime_type("invalid/type")

      # Extension mapping
      {:ok, extensions} = Kreuzberg.UtilityAPI.get_extensions_for_mime("application/pdf")

      # Embedding presets
      {:ok, presets} = Kreuzberg.UtilityAPI.list_embedding_presets()
      {:ok, preset} = Kreuzberg.UtilityAPI.get_embedding_preset("balanced")

      # Error classification
      atom = Kreuzberg.UtilityAPI.classify_error("File not found")
  """

  alias Kreuzberg.Native

  # Compiled regex patterns for error classification
  @io_error_regex ~r/(io|file|not\s+found|does\s+not\s+exist|permission|denied)/i
  @invalid_format_regex ~r/(invalid|unsupported|format|corrupted|damaged)/i
  @invalid_config_regex ~r/(config|configuration|option|parameter)/i
  @ocr_error_regex ~r/(ocr|optical\s+character|recognition)/i
  @extraction_error_regex ~r/(extract|extraction)/i

  @doc """
  Detect the MIME type of binary data using content inspection.

  Analyzes the binary content to determine the file format, supporting a wide range
  of document and image formats. This is more reliable than extension-based detection
  for files that may have incorrect extensions.

  ## Parameters

    * `data` - Binary data to analyze (any document or image format)

  ## Returns

    * `{:ok, mime_type}` - Detected MIME type as a string (e.g., "application/pdf")
    * `{:error, reason}` - Error if detection fails

  ## Examples

      iex> pdf_binary = File.read!("document.pdf")
      iex> {:ok, mime} = Kreuzberg.UtilityAPI.detect_mime_type(pdf_binary)
      iex> mime
      "application/pdf"

      iex> image_binary = File.read!("photo.jpg")
      iex> {:ok, mime} = Kreuzberg.UtilityAPI.detect_mime_type(image_binary)
      iex> mime
      "image/jpeg"
  """
  @spec detect_mime_type(binary()) :: {:ok, String.t()} | {:error, String.t()}
  def detect_mime_type(data) when is_binary(data) do
    case Native.detect_mime_type(data) do
      {:ok, mime_type} -> {:ok, mime_type}
      {:error, reason} -> {:error, reason}
    end
  end

  @doc """
  Detect the MIME type of a file using its path and extension.

  Uses file extension and optional content inspection to determine the file format.
  Faster than binary content analysis but may be less reliable for files with
  incorrect extensions.

  ## Parameters

    * `path` - File path as a string or Path.t()

  ## Returns

    * `{:ok, mime_type}` - Detected MIME type as a string
    * `{:error, reason}` - Error if detection fails

  ## Examples

      iex> {:ok, mime} = Kreuzberg.UtilityAPI.detect_mime_type_from_path("document.pdf")
      iex> mime
      "application/pdf"

      iex> {:ok, mime} = Kreuzberg.UtilityAPI.detect_mime_type_from_path("spreadsheet.xlsx")
      iex> mime
      "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
  """
  @spec detect_mime_type_from_path(String.t() | Path.t()) ::
          {:ok, String.t()} | {:error, String.t()}
  def detect_mime_type_from_path(path) do
    string_path = to_string(path)

    case Native.detect_mime_type_from_path(string_path) do
      {:ok, mime_type} -> {:ok, mime_type}
      {:error, reason} -> {:error, reason}
    end
  end

  @doc """
  Validate that a MIME type string is supported by Kreuzberg.

  Checks if the provided MIME type is in the list of supported formats that can be
  processed by Kreuzberg extractors.

  ## Parameters

    * `mime_type` - MIME type string to validate (e.g., "application/pdf")

  ## Returns

    * `{:ok, mime_type}` - Returns the MIME type if valid
    * `{:error, reason}` - Error if MIME type is not supported

  ## Examples

      iex> {:ok, _} = Kreuzberg.UtilityAPI.validate_mime_type("application/pdf")

      iex> {:error, _} = Kreuzberg.UtilityAPI.validate_mime_type("application/invalid")

      iex> {:ok, _} = Kreuzberg.UtilityAPI.validate_mime_type("image/jpeg")
  """
  @spec validate_mime_type(String.t()) :: {:ok, String.t()} | {:error, String.t()}
  def validate_mime_type(mime_type) when is_binary(mime_type) do
    case Native.validate_mime_type(mime_type) do
      {:ok, validated_mime} -> {:ok, validated_mime}
      {:error, reason} -> {:error, reason}
    end
  end

  @doc """
  Get all file extensions associated with a given MIME type.

  Maps a MIME type to its commonly used file extensions, which can be useful for
  file naming, validation, or user interface purposes.

  ## Parameters

    * `mime_type` - MIME type string (e.g., "application/pdf")

  ## Returns

    * `{:ok, extensions}` - List of file extensions (without dot, e.g., ["pdf"])
    * `{:error, reason}` - Error if MIME type is not found

  ## Examples

      iex> {:ok, exts} = Kreuzberg.UtilityAPI.get_extensions_for_mime("application/pdf")
      iex> exts
      ["pdf"]

      iex> {:ok, exts} = Kreuzberg.UtilityAPI.get_extensions_for_mime("image/jpeg")
      iex> exts
      ["jpg", "jpeg"]

      iex> {:ok, exts} = Kreuzberg.UtilityAPI.get_extensions_for_mime("text/plain")
      iex> exts
      ["txt"]
  """
  @spec get_extensions_for_mime(String.t()) :: {:ok, [String.t()]} | {:error, String.t()}
  def get_extensions_for_mime(mime_type) when is_binary(mime_type) do
    case Native.get_extensions_for_mime(mime_type) do
      {:ok, extensions} when is_list(extensions) -> {:ok, extensions}
      {:error, reason} -> {:error, reason}
    end
  end

  @doc """
  List all available embedding model presets.

  Returns the names of all embedding presets configured in Kreuzberg, which can be
  used with the embedding configuration options during extraction.

  ## Returns

    * `{:ok, presets}` - List of preset names as strings
    * `{:error, reason}` - Error if retrieval fails

  ## Examples

      iex> {:ok, presets} = Kreuzberg.UtilityAPI.list_embedding_presets()
      iex> presets
      ["balanced", "fast", "quality", "multilingual"]

      iex> Enum.member?(presets, "balanced")
      true
  """
  @spec list_embedding_presets() :: {:ok, [String.t()]} | {:error, String.t()}
  def list_embedding_presets do
    case Native.list_embedding_presets() do
      {:ok, presets} when is_list(presets) -> {:ok, presets}
      {:error, reason} -> {:error, reason}
    end
  end

  @doc """
  Get detailed information about a specific embedding preset.

  Retrieves comprehensive details about a named embedding preset, including model
  information, chunk configuration, and dimensionality.

  ## Parameters

    * `preset_name` - Name of the embedding preset (e.g., "fast", "balanced", "quality", "multilingual")

  ## Returns

    * `{:ok, preset_info}` - Map containing preset details with keys:
      * `"name"` - Preset name
      * `"chunk_size"` - Chunk size in tokens for processing
      * `"overlap"` - Chunk overlap in tokens
      * `"dimensions"` - Embedding vector dimension
      * `"description"` - Human-readable description
    * `{:error, reason}` - Error if preset not found

  ## Examples

      iex> {:ok, preset} = Kreuzberg.UtilityAPI.get_embedding_preset("fast")
      iex> preset["name"]
      "fast"
      iex> preset["dimensions"]
      384

      iex> {:ok, preset} = Kreuzberg.UtilityAPI.get_embedding_preset("quality")
      iex> is_map(preset)
      true
      iex> preset["chunk_size"]
      512

      iex> {:error, _} = Kreuzberg.UtilityAPI.get_embedding_preset("nonexistent")
  """
  @spec get_embedding_preset(String.t()) :: {:ok, map()} | {:error, String.t()}
  def get_embedding_preset(preset_name) when is_binary(preset_name) do
    case Native.get_embedding_preset(preset_name) do
      {:ok, preset_map} when is_map(preset_map) -> {:ok, preset_map}
      {:error, reason} -> {:error, reason}
    end
  end

  @doc """
  Classify an error message into a semantic error category.

  Analyzes error messages using pattern matching and heuristics to categorize them
  into predefined error types, useful for error handling and user feedback.

  ## Parameters

    * `error_message` - Error message string to classify

  ## Returns

    * `error_atom` - Atom representing the error category

  ## Error Categories

    * `:io_error` - File I/O related errors (file not found, permission denied, etc.)
    * `:invalid_format` - File format errors (corrupted files, unsupported formats, etc.)
    * `:invalid_config` - Configuration or parameter errors
    * `:ocr_error` - OCR engine or processing errors
    * `:extraction_error` - General extraction failures
    * `:unknown_error` - Errors that don't match other categories

  ## Examples

      iex> Kreuzberg.UtilityAPI.classify_error("File not found: /path/to/file.pdf")
      :io_error

      iex> Kreuzberg.UtilityAPI.classify_error("Invalid PDF format")
      :invalid_format

      iex> Kreuzberg.UtilityAPI.classify_error("OCR engine failed")
      :ocr_error

      iex> Kreuzberg.UtilityAPI.classify_error("Unknown error occurred")
      :unknown_error
  """
  @spec classify_error(String.t()) :: atom()
  def classify_error(error_message) when is_binary(error_message) do
    reason_lower = String.downcase(error_message)

    cond do
      Regex.match?(@io_error_regex, reason_lower) ->
        :io_error

      Regex.match?(@invalid_format_regex, reason_lower) ->
        :invalid_format

      Regex.match?(@invalid_config_regex, reason_lower) ->
        :invalid_config

      Regex.match?(@ocr_error_regex, reason_lower) ->
        :ocr_error

      Regex.match?(@extraction_error_regex, reason_lower) ->
        :extraction_error

      true ->
        :unknown_error
    end
  end

  @doc """
  Get information about all error categories.

  Returns a structured map describing all error classification categories that can be
  returned by the error classification system.

  ## Returns

    * `{:ok, error_details}` - Map where keys are error category atoms and values are
      descriptions and example patterns for each category

  ## Examples

      iex> {:ok, details} = Kreuzberg.UtilityAPI.get_error_details()
      iex> is_map(details)
      true
      iex> Map.has_key?(details, :io_error)
      true
      iex> details[:io_error]["examples"]
      ["File not found", "Permission denied", "No such file or directory"]
  """
  @spec get_error_details() :: {:ok, map()} | {:error, String.t()}
  def get_error_details do
    {:ok,
     %{
       io_error: %{
         "name" => "IO Error",
         "description" => "File I/O related errors such as file not found or permission denied",
         "examples" => ["File not found", "Permission denied", "No such file or directory"]
       },
       invalid_format: %{
         "name" => "Invalid Format",
         "description" => "File format errors including corrupted files or unsupported formats",
         "examples" => ["Invalid PDF format", "Corrupted file", "Unsupported format"]
       },
       invalid_config: %{
         "name" => "Invalid Configuration",
         "description" => "Configuration or parameter validation errors",
         "examples" => [
           "Invalid configuration",
           "Invalid parameter",
           "Unknown option"
         ]
       },
       ocr_error: %{
         "name" => "OCR Error",
         "description" => "OCR engine or processing errors",
         "examples" => ["OCR failed", "OCR timeout", "Recognition failed"]
       },
       extraction_error: %{
         "name" => "Extraction Error",
         "description" => "General extraction and processing failures",
         "examples" => ["Extraction failed", "Processing error"]
       },
       unknown_error: %{
         "name" => "Unknown Error",
         "description" => "Errors that don't match other categories",
         "examples" => ["Unexpected error", "Internal error"]
       }
     }}
  end
end
