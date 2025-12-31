defmodule Kreuzberg.ExtractionConfig do
  alias Kreuzberg.Native

  @moduledoc """
  Configuration structure for document extraction operations.

  Provides options for controlling extraction behavior including caching, quality processing,
  OCR, chunking, language detection, and post-processing. This module defines the configuration
  schema and provides validation utilities to ensure configurations are valid before passing
  them to the Rust extraction engine.

  ## Configuration Fields

  ### Boolean Flags (Top-level)

    * `:use_cache` - Enable result caching (default: true)
    * `:enable_quality_processing` - Enable quality post-processing (default: false)
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
  - `enable_quality_processing`: false - Quality processing is disabled by default (can be enabled as needed)
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
        "enable_quality_processing" => false,
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
    enable_quality_processing: false,
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
      "chunking" => normalize_nested_config(config.chunking),
      "ocr" => normalize_nested_config(config.ocr),
      "language_detection" => normalize_nested_config(config.language_detection),
      "postprocessor" => normalize_nested_config(config.postprocessor),
      "images" => normalize_nested_config(config.images),
      "pages" => normalize_nested_config(config.pages),
      "token_reduction" => normalize_nested_config(config.token_reduction),
      "keywords" => normalize_keywords_config(config.keywords),
      "pdf_options" => normalize_nested_config(config.pdf_options),
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

  @doc false
  defp normalize_nested_config(nil), do: nil

  @doc false
  defp normalize_nested_config(config) when is_map(config) do
    normalize_map_keys(config)
  end

  @doc false
  defp normalize_keywords_config(nil), do: nil

  @doc false
  defp normalize_keywords_config(keywords_config) when is_map(keywords_config) do
    # Normalize the keys and add defaults if not present
    # The Rust backend requires algorithm, max_keywords, min_score, and ngram_range
    normalized = normalize_map_keys(keywords_config)

    # Add default algorithm if not present (yake is the default)
    normalized =
      if Map.has_key?(normalized, "algorithm") do
        normalized
      else
        Map.put(normalized, "algorithm", "yake")
      end

    # Add default max_keywords if not present
    normalized =
      if Map.has_key?(normalized, "max_keywords") do
        normalized
      else
        Map.put(normalized, "max_keywords", 10)
      end

    # Add default min_score if not present
    normalized =
      if Map.has_key?(normalized, "min_score") do
        normalized
      else
        Map.put(normalized, "min_score", 0.0)
      end

    # Add default ngram_range [1, 3] if not present
    normalized =
      if Map.has_key?(normalized, "ngram_range") do
        normalized
      else
        Map.put(normalized, "ngram_range", [1, 3])
      end

    normalized
  end

  @doc false
  defp normalize_keywords_config(other), do: other

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
         :ok <- validate_chunking_config(config.chunking),
         :ok <- validate_nested_field(config.ocr, "ocr"),
         :ok <- validate_ocr_config(config.ocr),
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

  @doc """
  Load an ExtractionConfig from a file.

  Supports TOML, YAML, and JSON configuration file formats.
  The file format is automatically detected based on the file extension
  or file contents.

  ## Parameters

    * `file_path` - Path to the configuration file (String or Path.t())

  ## Returns

    * `{:ok, config}` - Successfully loaded configuration as a struct
    * `{:error, reason}` - Failed to load or parse the configuration file

  ## Supported Formats

    * `.toml` - TOML format (e.g., `kreuzberg.toml`)
    * `.yaml`, `.yml` - YAML format (e.g., `kreuzberg.yaml`)
    * `.json` - JSON format (e.g., `kreuzberg.json`)

  ## Examples

      iex> Kreuzberg.ExtractionConfig.from_file("kreuzberg.toml")
      {:ok, %Kreuzberg.ExtractionConfig{...}}

      iex> Kreuzberg.ExtractionConfig.from_file("/etc/config/extraction.yaml")
      {:ok, config}

      iex> Kreuzberg.ExtractionConfig.from_file("/nonexistent/file.toml")
      {:error, "File not found: /nonexistent/file.toml"}
  """
  @spec from_file(String.t() | Path.t()) :: {:ok, t()} | {:error, String.t()}
  def from_file(file_path) do
    file_path_str = to_string(file_path)

    case Native.config_from_file(file_path_str) do
      {:ok, json_str} ->
        parse_config_json(json_str)

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Discover and load an ExtractionConfig by searching directories.

  Searches the current working directory and all parent directories for
  a configuration file in the following order:
  1. `kreuzberg.toml`
  2. `kreuzberg.yaml`
  3. `kreuzberg.yml`
  4. `kreuzberg.json`

  Returns the first configuration file found.

  ## Returns

    * `{:ok, config}` - Successfully discovered and loaded configuration
    * `{:error, :not_found}` - No configuration file found in directory tree
    * `{:error, reason}` - Error loading or parsing the configuration file

  ## Examples

      # With kreuzberg.toml in current directory
      iex> Kreuzberg.ExtractionConfig.discover()
      {:ok, %Kreuzberg.ExtractionConfig{...}}

      # With kreuzberg.yaml in a parent directory
      iex> Kreuzberg.ExtractionConfig.discover()
      {:ok, config}

      # When no config file exists
      iex> Kreuzberg.ExtractionConfig.discover()
      {:error, :not_found}
  """
  @spec discover() :: {:ok, t()} | {:error, :not_found | String.t()}
  def discover do
    case Native.config_discover() do
      {:ok, json_str} ->
        parse_config_json(json_str)

      {:error, :not_found} ->
        {:error, :not_found}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # Private helper to parse JSON config returned from Rust NIFs
  @doc false
  defp parse_config_json(json_str) do
    case Jason.decode(json_str) do
      {:ok, config_map} ->
        case from_map(config_map) do
          {:ok, config} -> {:ok, config}
          {:error, reason} -> {:error, "Invalid configuration structure: #{reason}"}
        end

      {:error, _reason} ->
        {:error, "Failed to parse configuration JSON"}
    end
  end

  # Private helper to convert a map to an ExtractionConfig struct
  @doc false
  defp from_map(map) when is_map(map) do
    config = %__MODULE__{
      chunking: Map.get(map, "chunking"),
      ocr: Map.get(map, "ocr"),
      language_detection: Map.get(map, "language_detection"),
      postprocessor: Map.get(map, "postprocessor"),
      images: Map.get(map, "images"),
      pages: Map.get(map, "pages"),
      token_reduction: Map.get(map, "token_reduction"),
      keywords: Map.get(map, "keywords"),
      pdf_options: Map.get(map, "pdf_options"),
      use_cache: Map.get(map, "use_cache", true),
      enable_quality_processing: Map.get(map, "enable_quality_processing", true),
      force_ocr: Map.get(map, "force_ocr", false)
    }

    {:ok, config}
  rescue
    _e -> {:error, "Failed to create config struct"}
  end

  defp from_map(_), do: {:error, "Configuration must be a map"}

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

  @doc false
  defp validate_chunking_config(nil), do: :ok

  @doc false
  defp validate_chunking_config(config) when is_map(config) do
    with :ok <- validate_positive_integer(config, "max_chars"),
         :ok <- validate_positive_integer(config, "max_overlap"),
         :ok <- validate_overlap_not_exceeding_max_chars(config) do
      :ok
    end
  end

  @doc false
  defp validate_ocr_config(nil), do: :ok

  @doc false
  defp validate_ocr_config(config) when is_map(config) do
    with :ok <- validate_confidence_range(config),
         :ok <- validate_dpi_range(config) do
      :ok
    end
  end

  @doc false
  defp validate_positive_integer(config, key) do
    case Map.get(config, key) || Map.get(config, String.to_atom(key)) do
      nil ->
        :ok

      value when is_integer(value) and value > 0 ->
        :ok

      value when is_integer(value) and value <= 0 ->
        {:error, "Field '#{key}' must be a positive integer, got: #{value}"}

      value ->
        {:error, "Field '#{key}' must be a positive integer, got: #{type_name(value)}"}
    end
  end

  @doc false
  defp validate_overlap_not_exceeding_max_chars(config) do
    max_chars = Map.get(config, "max_chars") || Map.get(config, :max_chars)
    max_overlap = Map.get(config, "max_overlap") || Map.get(config, :max_overlap)

    cond do
      is_nil(max_chars) or is_nil(max_overlap) ->
        :ok

      is_integer(max_overlap) and is_integer(max_chars) and max_overlap > max_chars ->
        {:error, "Field 'max_overlap' (#{max_overlap}) cannot exceed 'max_chars' (#{max_chars})"}

      true ->
        :ok
    end
  end

  @doc false
  defp validate_confidence_range(config) do
    confidence = Map.get(config, "confidence") || Map.get(config, :confidence)

    case confidence do
      nil ->
        :ok

      value when is_number(value) and value >= 0.0 and value <= 1.0 ->
        :ok

      value when is_number(value) ->
        {:error, "Field 'confidence' must be between 0.0 and 1.0, got: #{value}"}

      value ->
        {:error, "Field 'confidence' must be a number between 0.0 and 1.0, got: #{type_name(value)}"}
    end
  end

  @doc false
  defp validate_dpi_range(config) do
    dpi = Map.get(config, "dpi") || Map.get(config, :dpi)

    case dpi do
      nil ->
        :ok

      value when is_integer(value) and value > 0 and value <= 2400 ->
        :ok

      value when is_integer(value) and value <= 0 ->
        {:error, "Field 'dpi' must be a positive integer, got: #{value}"}

      value when is_integer(value) and value > 2400 ->
        {:error, "Field 'dpi' must be at most 2400, got: #{value}"}

      value ->
        {:error, "Field 'dpi' must be a positive integer, got: #{type_name(value)}"}
    end
  end
end
