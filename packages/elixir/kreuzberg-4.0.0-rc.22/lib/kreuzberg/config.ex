defmodule Kreuzberg.ExtractionConfig do
  @moduledoc """
  Configuration structure for document extraction operations.

  Provides options for controlling extraction behavior including caching, quality processing,
  OCR, chunking, language detection, and post-processing. This module defines the configuration
  schema and provides validation utilities to ensure configurations are valid before passing
  them to the Rust extraction engine.

  ## Configuration Fields

  ### Boolean Flags (Top-level)

    * `:use_cache` - Enable result caching (default: true)
    * `:enable_quality_processing` - Enable quality post-processing (default: true)
    * `:force_ocr` - Force OCR even for searchable PDFs (default: false)

  ### Nested Configuration Maps (Optional)

    * `:chunking` - Text chunking configuration with options like chunk_size, overlap, etc.
    * `:ocr` - OCR backend configuration with settings for language, PSM mode, etc.
    * `:language_detection` - Language detection settings for multi-language content
    * `:postprocessor` - Post-processor configuration for cleaning/formatting extracted text
    * `:images` - Image extraction configuration (quality, format, preprocessing options)
    * `:pages` - Page-level extraction configuration (which pages to extract, etc.)
    * `:token_reduction` - Token reduction settings for optimizing output size
    * `:keywords` - Keyword extraction configuration
    * `:pdf_options` - PDF-specific options (requires pdf feature to be enabled)

  ## Default Values

  All boolean flags default to reasonable values:
  - `use_cache`: true - Caching is enabled by default for better performance
  - `enable_quality_processing`: true - Quality processing is enabled by default
  - `force_ocr`: false - OCR is only used when necessary (searchable PDFs bypass OCR)

  All nested configurations default to nil, allowing the Rust implementation to apply
  its own defaults.

  ## Field Validation

  The `validate/1` function ensures:
  - Boolean fields are actually booleans
  - Nested configurations are maps or nil
  - No invalid field names are used

  ## Examples

      # Create config with chunking enabled
      iex> config = %Kreuzberg.ExtractionConfig{
      ...>   chunking: %{"enabled" => true, "chunk_size" => 1024},
      ...>   use_cache: true
      ...> }
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:ok, config}

      # Create config that forces OCR
      iex> config = %Kreuzberg.ExtractionConfig{
      ...>   force_ocr: true,
      ...>   enable_quality_processing: true
      ...> }
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:ok, config}

      # Validate invalid configuration (non-boolean field)
      iex> config = %Kreuzberg.ExtractionConfig{use_cache: "yes"}
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:error, "Field 'use_cache' must be a boolean"}

      # Convert to map for NIF
      iex> config = %Kreuzberg.ExtractionConfig{chunking: %{"size" => 512}}
      iex> Kreuzberg.ExtractionConfig.to_map(config)
      %{
        "chunking" => %{"size" => 512},
        "ocr" => nil,
        "language_detection" => nil,
        "postprocessor" => nil,
        "images" => nil,
        "pages" => nil,
        "token_reduction" => nil,
        "keywords" => nil,
        "pdf_config" => nil,
        "use_cache" => true,
        "enable_quality_processing" => true,
        "force_ocr" => false
      }
  """

  @type config_map :: %{String.t() => any()}

  @type nested_config :: config_map | nil

  @type t :: %__MODULE__{
          chunking: nested_config,
          ocr: nested_config,
          language_detection: nested_config,
          postprocessor: nested_config,
          images: nested_config,
          pages: nested_config,
          token_reduction: nested_config,
          keywords: nested_config,
          pdf_options: nested_config,
          use_cache: boolean(),
          enable_quality_processing: boolean(),
          force_ocr: boolean()
        }

  defstruct [
    :chunking,
    :ocr,
    :language_detection,
    :postprocessor,
    :images,
    :pages,
    :token_reduction,
    :keywords,
    :pdf_options,
    use_cache: true,
    enable_quality_processing: true,
    force_ocr: false
  ]

  @doc """
  Converts an ExtractionConfig struct to a map for NIF serialization.

  Returns a map containing all configuration fields, both boolean flags and
  nested configurations. Serializes all values including nil for complete
  representation.

  ## Parameters

    * `config` - An `ExtractionConfig` struct, a plain map, nil, or a keyword list

  ## Returns

  A map with string keys representing the configuration options. All fields
  are included, allowing the Rust side to override with provided values.

  ## Field Descriptions

    * `"chunking"` - Text chunking configuration (map or nil)
    * `"ocr"` - OCR backend configuration (map or nil)
    * `"language_detection"` - Language detection settings (map or nil)
    * `"postprocessor"` - Post-processor configuration (map or nil)
    * `"images"` - Image extraction configuration (map or nil)
    * `"pages"` - Page-level extraction configuration (map or nil)
    * `"token_reduction"` - Token reduction settings (map or nil)
    * `"keywords"` - Keyword extraction configuration (map or nil)
    * `"pdf_options"` - PDF-specific options (map or nil)
    * `"use_cache"` - Enable caching (boolean)
    * `"enable_quality_processing"` - Enable quality processing (boolean)
    * `"force_ocr"` - Force OCR usage (boolean)

  ## Examples

      iex> config = %Kreuzberg.ExtractionConfig{chunking: %{"size" => 512}}
      iex> Kreuzberg.ExtractionConfig.to_map(config)
      %{
        "chunking" => %{"size" => 512},
        "ocr" => nil,
        "language_detection" => nil,
        "postprocessor" => nil,
        "images" => nil,
        "pages" => nil,
        "token_reduction" => nil,
        "keywords" => nil,
        "pdf_options" => nil,
        "use_cache" => true,
        "enable_quality_processing" => true,
        "force_ocr" => false
      }

      iex> config = %Kreuzberg.ExtractionConfig{}
      iex> Kreuzberg.ExtractionConfig.to_map(config)
      %{
        "chunking" => nil,
        "ocr" => nil,
        "language_detection" => nil,
        "postprocessor" => nil,
        "images" => nil,
        "pages" => nil,
        "token_reduction" => nil,
        "keywords" => nil,
        "pdf_options" => nil,
        "use_cache" => true,
        "enable_quality_processing" => true,
        "force_ocr" => false
      }

      iex> Kreuzberg.ExtractionConfig.to_map(nil)
      nil

      iex> Kreuzberg.ExtractionConfig.to_map(%{"use_cache" => false})
      %{"use_cache" => false}
  """
  @spec to_map(t() | map() | nil | list()) :: map() | nil
  def to_map(nil), do: nil

  def to_map(map) when is_map(map) and not is_struct(map) do
    normalize_map_keys(map)
  end

  def to_map(%__MODULE__{} = config) do
    %{
      "chunking" => config.chunking,
      "ocr" => config.ocr,
      "language_detection" => config.language_detection,
      "postprocessor" => config.postprocessor,
      "images" => config.images,
      "pages" => config.pages,
      "token_reduction" => config.token_reduction,
      "keywords" => config.keywords,
      "pdf_options" => config.pdf_options,
      "use_cache" => config.use_cache,
      "enable_quality_processing" => config.enable_quality_processing,
      "force_ocr" => config.force_ocr
    }
  end

  def to_map(keyword_list) when is_list(keyword_list) do
    keyword_list
    |> Map.new()
    |> to_map()
  end

  @doc false
  defp normalize_map_keys(map) when is_map(map) do
    map
    |> Enum.reduce(%{}, fn
      {key, value}, acc when is_binary(key) ->
        Map.put(acc, key, value)

      {key, value}, acc ->
        string_key = if is_atom(key), do: Atom.to_string(key), else: to_string(key)
        Map.put(acc, string_key, value)
    end)
  end

  @doc """
  Validates an ExtractionConfig for correct field types and values.

  Ensures that:
  - Boolean fields (use_cache, enable_quality_processing, force_ocr) are actually booleans
  - Nested configuration fields are maps or nil
  - All values are valid according to the configuration schema

  This function is useful for early validation before passing configuration
  to the extraction functions.

  ## Parameters

    * `config` - An `ExtractionConfig` struct to validate

  ## Returns

    * `{:ok, config}` - If the configuration is valid
    * `{:error, reason}` - If validation fails, with a descriptive error message

  ## Examples

      iex> config = %Kreuzberg.ExtractionConfig{use_cache: true}
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:ok, config}

      iex> config = %Kreuzberg.ExtractionConfig{chunking: %{"size" => 1024}}
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:ok, config}

      iex> config = %Kreuzberg.ExtractionConfig{use_cache: "yes"}
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:error, "Field 'use_cache' must be a boolean, got: string"}

      iex> config = %Kreuzberg.ExtractionConfig{chunking: "invalid"}
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:error, "Field 'chunking' must be a map or nil, got: string"}

      iex> config = %Kreuzberg.ExtractionConfig{force_ocr: true, enable_quality_processing: true}
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:ok, config}
  """
  @spec validate(t()) :: {:ok, t()} | {:error, String.t()}
  def validate(%__MODULE__{} = config) do
    with :ok <- validate_boolean_field(config.use_cache, "use_cache"),
         :ok <-
           validate_boolean_field(config.enable_quality_processing, "enable_quality_processing"),
         :ok <- validate_boolean_field(config.force_ocr, "force_ocr"),
         :ok <- validate_nested_field(config.chunking, "chunking"),
         :ok <- validate_nested_field(config.ocr, "ocr"),
         :ok <- validate_nested_field(config.language_detection, "language_detection"),
         :ok <- validate_nested_field(config.postprocessor, "postprocessor"),
         :ok <- validate_nested_field(config.images, "images"),
         :ok <- validate_nested_field(config.pages, "pages"),
         :ok <- validate_nested_field(config.token_reduction, "token_reduction"),
         :ok <- validate_nested_field(config.keywords, "keywords"),
         :ok <- validate_nested_field(config.pdf_options, "pdf_options") do
      {:ok, config}
    end
  end

  @doc false
  defp validate_boolean_field(value, field_name) do
    if is_boolean(value) do
      :ok
    else
      {:error, "Field '#{field_name}' must be a boolean, got: #{type_name(value)}"}
    end
  end

  @doc false
  defp validate_nested_field(nil, _field_name), do: :ok

  @doc false
  defp validate_nested_field(value, field_name) do
    if is_map(value) do
      :ok
    else
      {:error, "Field '#{field_name}' must be a map or nil, got: #{type_name(value)}"}
    end
  end

  @doc false
  defp type_name(value) do
    cond do
      is_boolean(value) -> "boolean"
      is_integer(value) -> "integer"
      is_float(value) -> "float"
      is_binary(value) -> "string"
      is_list(value) -> "list"
      is_map(value) -> "map"
      is_atom(value) -> "atom"
      true -> "unknown"
    end
  end
end
