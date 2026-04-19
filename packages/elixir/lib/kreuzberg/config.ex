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
    * `:enable_quality_processing` - Enable quality post-processing (default: true)
    * `:force_ocr` - Force OCR even for searchable PDFs (default: false)
    * `:disable_ocr` - Disable OCR entirely — image files return empty content (default: false)

  ### Output Format Flags

    * `:output_format` - Content text format (default: "plain") - "plain", "markdown", "djot", "html"
    * `:result_format` - Result structure format (default: "unified") - "unified", "element_based"

  ### Nested Configuration Maps (Optional)

    * `:chunking` - Text chunking configuration with options like chunk_size, overlap, etc.
    * `:ocr` - OCR backend configuration with settings for language, PSM mode, etc.
      - Can include nested `:paddle_ocr_config` for PaddleOCR-specific settings
      - Can include nested `:element_config` for OCR element extraction settings
    * `:language_detection` - Language detection settings for multi-language content
    * `:postprocessor` - Post-processor configuration for cleaning/formatting extracted text
    * `:images` - Image extraction configuration (quality, format, preprocessing options)
    * `:pages` - Page-level extraction configuration (which pages to extract, etc.)
    * `:token_reduction` - Token reduction settings for optimizing output size
    * `:keywords` - Keyword extraction configuration
    * `:pdf_options` - PDF-specific options (requires pdf feature to be enabled)
    * `:html_options` - HTML to Markdown conversion options (quality, format, preprocessing options)
    * `:layout` - Layout detection configuration (confidence_threshold, apply_heuristics, table_model)
    * `:acceleration` - GPU acceleration configuration (provider, device_id)
    * `:security_limits` - Security limits for archive extraction (max sizes, compression ratio, etc.)
    * `:email` - Email extraction configuration (msg_fallback_codepage)
    * `:content_filter` - Content filter configuration (include_headers, include_footers, strip_repeating_text, include_watermarks)
    * `:max_concurrent_extractions` - Maximum concurrent extractions in batch operations (positive integer or nil)
    * `:cache_namespace` - Cache namespace for tenant isolation (string or nil)
    * `:cache_ttl_secs` - Per-request cache TTL in seconds (non-negative integer or nil)
    * `:extraction_timeout_secs` - Per-request extraction timeout in seconds; when exceeded, extraction is cancelled (non-negative integer or nil)

  ## Default Values

  All boolean flags default to reasonable values:
  - `use_cache`: true - Caching is enabled by default for better performance
  - `enable_quality_processing`: true - Quality processing is enabled by default for better extraction results
  - `force_ocr`: false - OCR is only used when necessary (searchable PDFs bypass OCR)
  - `disable_ocr`: false - OCR is available when needed

  Format defaults:
  - `output_format`: "plain" - Raw extracted text (no formatting)
  - `result_format`: "unified" - All content in unified content field

  All nested configurations default to nil, allowing the Rust implementation to apply
  its own defaults.

  ## Field Validation

  The `validate/1` function ensures:
  - Boolean fields are actually booleans
  - Format fields are valid enum values
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

      # Create config with markdown output format
      iex> config = %Kreuzberg.ExtractionConfig{
      ...>   output_format: "markdown",
      ...>   result_format: "unified"
      ...> }
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:ok, config}

      # Create config that forces OCR with element-based result format
      iex> config = %Kreuzberg.ExtractionConfig{
      ...>   force_ocr: true,
      ...>   result_format: "element_based",
      ...>   enable_quality_processing: true
      ...> }
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:ok, config}

      # Validate invalid configuration (non-boolean field)
      iex> config = %Kreuzberg.ExtractionConfig{use_cache: "yes"}
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:error, "Field 'use_cache' must be a boolean, got: string"}

      # Validate invalid format
      iex> config = %Kreuzberg.ExtractionConfig{output_format: "invalid"}
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:error, "Field 'output_format' must be one of: plain, text, markdown, md, djot, html, got: invalid"}

      # Convert to map for NIF
      iex> config = %Kreuzberg.ExtractionConfig{chunking: %{"size" => 512}}
      iex> Kreuzberg.ExtractionConfig.to_map(config)
      %{
        "acceleration" => nil,
        "cache_namespace" => nil,
        "cache_ttl_secs" => nil,
        "chunking" => %{"size" => 512},
        "concurrency" => nil,
        "content_filter" => nil,
        "email" => nil,
        "disable_ocr" => false,
        "enable_quality_processing" => true,
        "extraction_timeout_secs" => nil,
        "force_ocr" => false,
        "force_ocr_pages" => nil,
        "html_options" => nil,
        "images" => nil,
        "include_document_structure" => false,
        "keywords" => nil,
        "language_detection" => nil,
        "layout" => nil,
        "max_archive_depth" => 3,
        "max_concurrent_extractions" => nil,
        "ocr" => nil,
        "output_format" => "plain",
        "pages" => nil,
        "pdf_options" => nil,
        "postprocessor" => nil,
        "result_format" => "unified",
        "security_limits" => nil,
        "token_reduction" => nil,
        "tree_sitter" => nil,
        "use_cache" => true
      }
  """

  @type config_map :: %{String.t() => any()}

  @type nested_config :: config_map | nil

  @type layout_config ::
          %{
            confidence_threshold: float() | nil,
            apply_heuristics: boolean(),
            table_model: String.t() | nil
          }
          | nil

  @type output_format :: String.t()

  @type result_format :: String.t()

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
          max_concurrent_extractions: non_neg_integer() | nil,
          html_options: config_map | nil,
          html_output: nested_config,
          layout: layout_config,
          acceleration: nested_config,
          security_limits: nested_config,
          email: nested_config,
          content_filter: nested_config,
          concurrency: nested_config,
          tree_sitter: Kreuzberg.TreeSitterConfig.t() | nested_config,
          use_cache: boolean(),
          enable_quality_processing: boolean(),
          force_ocr: boolean(),
          disable_ocr: boolean(),
          force_ocr_pages: [non_neg_integer()] | nil,
          output_format: output_format,
          result_format: result_format,
          include_document_structure: boolean(),
          cache_namespace: String.t() | nil,
          cache_ttl_secs: non_neg_integer() | nil,
          extraction_timeout_secs: non_neg_integer() | nil,
          max_archive_depth: non_neg_integer()
        }

  @derive Jason.Encoder
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
    :max_concurrent_extractions,
    :html_options,
    :html_output,
    :layout,
    :acceleration,
    :security_limits,
    :email,
    :content_filter,
    :concurrency,
    :tree_sitter,
    :cache_namespace,
    :cache_ttl_secs,
    :extraction_timeout_secs,
    use_cache: true,
    enable_quality_processing: true,
    force_ocr: false,
    disable_ocr: false,
    force_ocr_pages: nil,
    output_format: "plain",
    result_format: "unified",
    include_document_structure: false,
    max_archive_depth: 3
  ]

  @doc """
  Creates a new ExtractionConfig with all default values.

  ## Examples

      iex> config = Kreuzberg.ExtractionConfig.new()
      iex> config.use_cache
      true
  """
  @spec new() :: t()
  def new do
    %__MODULE__{}
  end

  @doc """
  Creates a new ExtractionConfig from keyword list or map.

  ## Parameters

    * `opts` - Keyword list or map (supports string keys from JSON)

  ## Examples

      iex> config = Kreuzberg.ExtractionConfig.new(use_cache: false)
      iex> config.use_cache
      false

      iex> config = Kreuzberg.ExtractionConfig.new(%{"output_format" => "markdown"})
      iex> config.output_format
      "markdown"
  """
  @spec new(keyword() | map()) :: t()
  def new(opts) when is_list(opts) do
    opts |> Map.new() |> new()
  end

  def new(opts) when is_map(opts) and not is_struct(opts) do
    normalized =
      opts
      |> Enum.reduce(%{}, fn
        {key, value}, acc when is_binary(key) ->
          atom_key =
            try do
              String.to_existing_atom(key)
            rescue
              ArgumentError -> key
            end

          Map.put(acc, atom_key, value)

        {key, value}, acc ->
          Map.put(acc, key, value)
      end)

    struct(__MODULE__, normalized)
  end

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
    * `"max_concurrent_extractions"` - Maximum concurrent extractions (positive integer or nil)
    * `"html_options"` - HTML to Markdown conversion options (map or nil)
    * `"layout"` - Layout detection configuration (map or nil)
    * `"include_document_structure"` - Include document structure in extraction (boolean)
    * `"use_cache"` - Enable caching (boolean)
    * `"enable_quality_processing"` - Enable quality processing (boolean)
    * `"force_ocr"` - Force OCR usage (boolean)
    * `"output_format"` - Content text format (string: "plain", "markdown", "djot", "html")
    * `"result_format"` - Result structure format (string: "unified", "element_based")

  ## Examples

      iex> config = %Kreuzberg.ExtractionConfig{chunking: %{"size" => 512}, output_format: "markdown"}
      iex> Kreuzberg.ExtractionConfig.to_map(config)
      %{
        "acceleration" => nil,
        "cache_namespace" => nil,
        "cache_ttl_secs" => nil,
        "chunking" => %{"size" => 512},
        "concurrency" => nil,
        "email" => nil,
        "disable_ocr" => false,
        "enable_quality_processing" => true,
        "extraction_timeout_secs" => nil,
        "force_ocr" => false,
        "force_ocr_pages" => nil,
        "html_options" => nil,
        "images" => nil,
        "include_document_structure" => false,
        "keywords" => nil,
        "language_detection" => nil,
        "layout" => nil,
        "max_archive_depth" => 3,
        "max_concurrent_extractions" => nil,
        "ocr" => nil,
        "output_format" => "markdown",
        "pages" => nil,
        "pdf_options" => nil,
        "postprocessor" => nil,
        "result_format" => "unified",
        "security_limits" => nil,
        "token_reduction" => nil,
        "tree_sitter" => nil,
        "use_cache" => true
      }

      iex> config = %Kreuzberg.ExtractionConfig{}
      iex> Kreuzberg.ExtractionConfig.to_map(config)
      %{
        "acceleration" => nil,
        "cache_namespace" => nil,
        "cache_ttl_secs" => nil,
        "chunking" => nil,
        "concurrency" => nil,
        "content_filter" => nil,
        "email" => nil,
        "disable_ocr" => false,
        "enable_quality_processing" => true,
        "extraction_timeout_secs" => nil,
        "force_ocr" => false,
        "force_ocr_pages" => nil,
        "html_options" => nil,
        "images" => nil,
        "include_document_structure" => false,
        "keywords" => nil,
        "language_detection" => nil,
        "layout" => nil,
        "max_archive_depth" => 3,
        "max_concurrent_extractions" => nil,
        "ocr" => nil,
        "output_format" => "plain",
        "pages" => nil,
        "pdf_options" => nil,
        "postprocessor" => nil,
        "result_format" => "unified",
        "security_limits" => nil,
        "token_reduction" => nil,
        "tree_sitter" => nil,
        "use_cache" => true
      }

      iex> Kreuzberg.ExtractionConfig.to_map(nil)
      nil

      iex> Kreuzberg.ExtractionConfig.to_map(%{"use_cache" => false, "output_format" => "markdown"})
      %{"use_cache" => false, "output_format" => "markdown"}
  """
  @spec to_map(t() | map() | nil | list()) :: map() | nil
  def to_map(nil), do: nil

  def to_map(map) when is_map(map) and not is_struct(map) do
    normalized = normalize_map_keys(map)

    case normalized do
      %{"ocr" => ocr} when is_map(ocr) -> Map.put(normalized, "ocr", Map.delete(ocr, "enabled"))
      _ -> normalized
    end
  end

  def to_map(%__MODULE__{} = config) do
    %{
      "chunking" => normalize_nested_config(config.chunking),
      "ocr" => normalize_ocr_config(config.ocr),
      "language_detection" => normalize_nested_config(config.language_detection),
      "postprocessor" => normalize_nested_config(config.postprocessor),
      "images" => normalize_nested_config(config.images),
      "pages" => normalize_nested_config(config.pages),
      "token_reduction" => normalize_nested_config(config.token_reduction),
      "keywords" => normalize_keywords_config(config.keywords),
      "pdf_options" => normalize_nested_config(config.pdf_options),
      "max_concurrent_extractions" => config.max_concurrent_extractions,
      "html_options" => normalize_nested_config(config.html_options),
      "layout" => normalize_layout_config(config.layout),
      "acceleration" => normalize_acceleration_config(config.acceleration),
      "security_limits" => normalize_nested_config(config.security_limits),
      "email" => normalize_email_config(config.email),
      "content_filter" => normalize_nested_config(config.content_filter),
      "concurrency" => normalize_concurrency_config(config.concurrency),
      "tree_sitter" => normalize_nested_config(config.tree_sitter),
      "use_cache" => config.use_cache,
      "enable_quality_processing" => config.enable_quality_processing,
      "force_ocr" => config.force_ocr,
      "disable_ocr" => config.disable_ocr,
      "force_ocr_pages" => config.force_ocr_pages,
      "output_format" => normalize_format_value(config.output_format),
      "result_format" => normalize_format_value(config.result_format),
      "include_document_structure" => config.include_document_structure,
      "cache_namespace" => config.cache_namespace,
      "cache_ttl_secs" => config.cache_ttl_secs,
      "extraction_timeout_secs" => config.extraction_timeout_secs,
      "max_archive_depth" => config.max_archive_depth
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
        normalized_value = normalize_map_keys_recursive(value)
        Map.put(acc, key, normalized_value)

      {key, value}, acc ->
        string_key = if is_atom(key), do: Atom.to_string(key), else: to_string(key)
        normalized_value = normalize_map_keys_recursive(value)
        Map.put(acc, string_key, normalized_value)
    end)
  end

  @doc false
  defp normalize_map_keys_recursive(value) when is_map(value) and not is_struct(value) do
    normalize_map_keys(value)
  end

  @doc false
  defp normalize_map_keys_recursive(value) when is_list(value) do
    Enum.map(value, &normalize_map_keys_recursive/1)
  end

  @doc false
  defp normalize_map_keys_recursive(value), do: value

  @doc false
  defp normalize_ocr_config(nil), do: nil

  @doc false
  defp normalize_ocr_config(config) when is_map(config) do
    config
    |> normalize_map_keys()
    |> Map.delete("enabled")
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

  @doc false
  defp normalize_layout_config(nil), do: nil

  @doc false
  defp normalize_layout_config(layout_config) when is_map(layout_config) do
    normalized = normalize_map_keys(layout_config)

    # Remove preset if present (no longer used)
    normalized = Map.delete(normalized, "preset")

    # Add default apply_heuristics if not present
    normalized =
      if Map.has_key?(normalized, "apply_heuristics") do
        normalized
      else
        Map.put(normalized, "apply_heuristics", true)
      end

    normalized
  end

  @doc false
  defp normalize_layout_config(other), do: other

  @doc false
  defp normalize_acceleration_config(nil), do: nil

  @doc false
  defp normalize_acceleration_config(accel_config) when is_map(accel_config) do
    normalized = normalize_map_keys(accel_config)

    # Add default provider if not present ("auto" is the default)
    normalized =
      if Map.has_key?(normalized, "provider") do
        normalized
      else
        Map.put(normalized, "provider", "auto")
      end

    # Add default device_id if not present (0 is the default)
    normalized =
      if Map.has_key?(normalized, "device_id") do
        normalized
      else
        Map.put(normalized, "device_id", 0)
      end

    normalized
  end

  @doc false
  defp normalize_acceleration_config(other), do: other

  @doc false
  defp normalize_email_config(nil), do: nil

  @doc false
  defp normalize_email_config(email_config) when is_map(email_config) do
    normalize_map_keys(email_config)
  end

  @doc false
  defp normalize_email_config(other), do: other

  @doc false
  defp normalize_concurrency_config(nil), do: nil

  @doc false
  defp normalize_concurrency_config(concurrency_config) when is_map(concurrency_config) do
    normalize_map_keys(concurrency_config)
  end

  @doc false
  defp normalize_concurrency_config(other), do: other

  @doc false
  defp normalize_format_value(value) when is_binary(value) do
    String.downcase(value)
  end

  @doc false
  defp normalize_format_value(value), do: value

  @doc """
  Validates an ExtractionConfig for correct field types and values.

  Ensures that:
  - Boolean fields (use_cache, enable_quality_processing, force_ocr) are actually booleans
  - Format fields (output_format, result_format) are valid enum values
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

      iex> config = %Kreuzberg.ExtractionConfig{output_format: "markdown", result_format: "unified"}
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:ok, config}

      iex> config = %Kreuzberg.ExtractionConfig{chunking: %{"size" => 1024}}
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:ok, config}

      iex> config = %Kreuzberg.ExtractionConfig{use_cache: "yes"}
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:error, "Field 'use_cache' must be a boolean, got: string"}

      iex> config = %Kreuzberg.ExtractionConfig{output_format: "invalid"}
      iex> Kreuzberg.ExtractionConfig.validate(config)
      {:error, "Field 'output_format' must be one of: plain, text, markdown, md, djot, html, got: invalid"}

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
         :ok <- validate_boolean_field(config.disable_ocr, "disable_ocr"),
         :ok <- validate_output_format(config.output_format),
         :ok <- validate_result_format(config.result_format),
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
         :ok <- validate_nested_field(config.pdf_options, "pdf_options"),
         :ok <- validate_max_concurrent_extractions(config.max_concurrent_extractions),
         :ok <- validate_nested_field(config.html_options, "html_options"),
         :ok <- validate_nested_field(config.layout, "layout"),
         :ok <- validate_layout_config(config.layout),
         :ok <- validate_nested_field(config.acceleration, "acceleration"),
         :ok <- validate_acceleration_config(config.acceleration),
         :ok <- validate_nested_field(config.security_limits, "security_limits"),
         :ok <- validate_nested_field(config.email, "email"),
         :ok <- validate_email_config(config.email),
         :ok <- validate_nested_field(config.content_filter, "content_filter"),
         :ok <- validate_nested_field(config.concurrency, "concurrency"),
         :ok <- validate_concurrency_config(config.concurrency),
         :ok <- validate_nested_field(config.tree_sitter, "tree_sitter") do
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

  Loading from a TOML file:

      Kreuzberg.ExtractionConfig.from_file("kreuzberg.toml")
      # => {:ok, %Kreuzberg.ExtractionConfig{...}}

  Loading from a YAML file:

      Kreuzberg.ExtractionConfig.from_file("/etc/config/extraction.yaml")
      # => {:ok, %Kreuzberg.ExtractionConfig{...}}

  Handling missing files:

      Kreuzberg.ExtractionConfig.from_file("/nonexistent/file.toml")
      # => {:error, "File not found: ..."}
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
      max_concurrent_extractions: Map.get(map, "max_concurrent_extractions"),
      html_options: Map.get(map, "html_options"),
      layout: Map.get(map, "layout"),
      acceleration: Map.get(map, "acceleration"),
      security_limits: Map.get(map, "security_limits"),
      email: Map.get(map, "email"),
      concurrency: Map.get(map, "concurrency"),
      tree_sitter: Map.get(map, "tree_sitter"),
      use_cache: Map.get(map, "use_cache", true),
      enable_quality_processing: Map.get(map, "enable_quality_processing", true),
      force_ocr: Map.get(map, "force_ocr", false),
      disable_ocr: Map.get(map, "disable_ocr", false),
      force_ocr_pages: Map.get(map, "force_ocr_pages"),
      output_format: Map.get(map, "output_format", "plain"),
      result_format: Map.get(map, "result_format", "unified"),
      cache_namespace: Map.get(map, "cache_namespace"),
      cache_ttl_secs: Map.get(map, "cache_ttl_secs"),
      extraction_timeout_secs: Map.get(map, "extraction_timeout_secs")
    }

    {:ok, config}
  rescue
    _e -> {:error, "Failed to create config struct"}
  end

  defp from_map(_), do: {:error, "Configuration must be a map"}

  @doc false
  defp validate_max_concurrent_extractions(nil), do: :ok

  @doc false
  defp validate_max_concurrent_extractions(value) when is_integer(value) and value > 0, do: :ok

  @doc false
  defp validate_max_concurrent_extractions(value) when is_integer(value) and value <= 0 do
    {:error, "Field 'max_concurrent_extractions' must be a positive integer, got: #{value}"}
  end

  @doc false
  defp validate_max_concurrent_extractions(value) do
    {:error,
     "Field 'max_concurrent_extractions' must be a positive integer or nil, got: #{type_name(value)}"}
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
  defp validate_output_format(value) when is_binary(value) do
    case String.downcase(value) do
      "plain" ->
        :ok

      "text" ->
        :ok

      "markdown" ->
        :ok

      "md" ->
        :ok

      "djot" ->
        :ok

      "html" ->
        :ok

      "json" ->
        :ok

      "structured" ->
        :ok

      _invalid ->
        {:error,
         "Field 'output_format' must be one of: plain, text, markdown, md, djot, html, json, structured, got: #{value}"}
    end
  end

  @doc false
  defp validate_output_format(value) do
    {:error, "Field 'output_format' must be a string, got: #{type_name(value)}"}
  end

  @doc false
  defp validate_result_format(value) when is_binary(value) do
    case String.downcase(value) do
      "unified" ->
        :ok

      "element_based" ->
        :ok

      "elementbased" ->
        :ok

      _invalid ->
        {:error, "Field 'result_format' must be one of: unified, element_based, got: #{value}"}
    end
  end

  @doc false
  defp validate_result_format(value) do
    {:error, "Field 'result_format' must be a string, got: #{type_name(value)}"}
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
         :ok <- validate_optional_boolean(config, "prepend_heading_context"),
         :ok <- validate_chunker_type(config),
         :ok <- validate_topic_threshold(config) do
      validate_overlap_not_exceeding_max_chars(config)
    end
  end

  @doc false
  defp validate_chunker_type(config) do
    value = Map.get(config, "chunker_type") || Map.get(config, :chunker_type)

    case value do
      nil ->
        :ok

      v when is_binary(v) ->
        case String.downcase(v) do
          "text" ->
            :ok

          "markdown" ->
            :ok

          "yaml" ->
            :ok

          "semantic" ->
            :ok

          _invalid ->
            {:error,
             "Field 'chunker_type' must be one of: text, markdown, yaml, semantic, got: #{v}"}
        end

      v ->
        {:error, "Field 'chunker_type' must be a string, got: #{type_name(v)}"}
    end
  end

  @doc false
  defp validate_topic_threshold(config) do
    value = Map.get(config, "topic_threshold") || Map.get(config, :topic_threshold)

    case value do
      nil ->
        :ok

      v when is_number(v) and v >= 0.0 and v <= 1.0 ->
        :ok

      v when is_number(v) ->
        {:error, "Field 'topic_threshold' must be between 0.0 and 1.0, got: #{v}"}

      v ->
        {:error,
         "Field 'topic_threshold' must be a number between 0.0 and 1.0, got: #{type_name(v)}"}
    end
  end

  @doc false
  defp validate_ocr_config(nil), do: :ok

  @doc false
  defp validate_ocr_config(config) when is_map(config) do
    with :ok <- validate_confidence_range(config) do
      validate_dpi_range(config)
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
  defp validate_optional_boolean(config, key) do
    value = Map.get(config, key) || Map.get(config, String.to_atom(key))

    case value do
      nil -> :ok
      v when is_boolean(v) -> :ok
      v -> {:error, "Field '#{key}' must be a boolean, got: #{type_name(v)}"}
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
        {:error,
         "Field 'confidence' must be a number between 0.0 and 1.0, got: #{type_name(value)}"}
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

  @doc false
  defp validate_layout_config(nil), do: :ok

  @doc false
  defp validate_layout_config(config) when is_map(config) do
    with :ok <- validate_layout_confidence_threshold(config),
         :ok <- validate_layout_apply_heuristics(config) do
      validate_layout_table_model(config)
    end
  end

  @doc false
  defp validate_layout_confidence_threshold(config) do
    threshold =
      case Map.fetch(config, "confidence_threshold") do
        {:ok, value} -> value
        :error -> Map.get(config, :confidence_threshold)
      end

    case threshold do
      nil ->
        :ok

      value when is_number(value) and value >= 0.0 and value <= 1.0 ->
        :ok

      value when is_number(value) ->
        {:error, "Field 'layout.confidence_threshold' must be between 0.0 and 1.0, got: #{value}"}

      value ->
        {:error,
         "Field 'layout.confidence_threshold' must be a number between 0.0 and 1.0, got: #{type_name(value)}"}
    end
  end

  @doc false
  defp validate_layout_apply_heuristics(config) do
    apply_heuristics =
      case Map.fetch(config, "apply_heuristics") do
        {:ok, value} -> value
        :error -> Map.get(config, :apply_heuristics)
      end

    case apply_heuristics do
      nil ->
        :ok

      value when is_boolean(value) ->
        :ok

      value ->
        {:error, "Field 'layout.apply_heuristics' must be a boolean, got: #{type_name(value)}"}
    end
  end

  @doc false
  defp validate_layout_table_model(config) do
    table_model =
      case Map.fetch(config, "table_model") do
        {:ok, value} -> value
        :error -> Map.get(config, :table_model)
      end

    case table_model do
      nil ->
        :ok

      value when is_binary(value) ->
        case String.downcase(value) do
          "tatr" ->
            :ok

          "slanet_wired" ->
            :ok

          "slanet_wireless" ->
            :ok

          "slanet_plus" ->
            :ok

          "slanet_auto" ->
            :ok

          _invalid ->
            {:error,
             "Field 'layout.table_model' must be one of: tatr, slanet_wired, slanet_wireless, slanet_plus, slanet_auto, got: #{value}"}
        end

      value ->
        {:error, "Field 'layout.table_model' must be a string, got: #{type_name(value)}"}
    end
  end

  @doc false
  defp validate_acceleration_config(nil), do: :ok

  @doc false
  defp validate_acceleration_config(config) when is_map(config) do
    with :ok <- validate_acceleration_provider(config) do
      validate_acceleration_device_id(config)
    end
  end

  @doc false
  defp validate_acceleration_provider(config) do
    provider = Map.get(config, "provider") || Map.get(config, :provider)

    case provider do
      nil ->
        :ok

      value when is_binary(value) ->
        case String.downcase(value) do
          "auto" ->
            :ok

          "cpu" ->
            :ok

          "coreml" ->
            :ok

          "cuda" ->
            :ok

          "tensorrt" ->
            :ok

          _invalid ->
            {:error,
             "Field 'acceleration.provider' must be one of: auto, cpu, coreml, cuda, tensorrt, got: #{value}"}
        end

      value ->
        {:error, "Field 'acceleration.provider' must be a string, got: #{type_name(value)}"}
    end
  end

  @doc false
  defp validate_acceleration_device_id(config) do
    device_id = Map.get(config, "device_id") || Map.get(config, :device_id)

    case device_id do
      nil ->
        :ok

      value when is_integer(value) and value >= 0 ->
        :ok

      value when is_integer(value) and value < 0 ->
        {:error, "Field 'acceleration.device_id' must be a non-negative integer, got: #{value}"}

      value ->
        {:error,
         "Field 'acceleration.device_id' must be a non-negative integer, got: #{type_name(value)}"}
    end
  end

  @doc false
  defp validate_concurrency_config(nil), do: :ok

  @doc false
  defp validate_concurrency_config(config) when is_map(config) do
    max_threads = Map.get(config, "max_threads") || Map.get(config, :max_threads)

    case max_threads do
      nil ->
        :ok

      value when is_integer(value) and value > 0 ->
        :ok

      value when is_integer(value) ->
        {:error, "Field 'concurrency.max_threads' must be a positive integer, got: #{value}"}

      value ->
        {:error,
         "Field 'concurrency.max_threads' must be a positive integer, got: #{type_name(value)}"}
    end
  end

  @doc false
  defp validate_email_config(nil), do: :ok

  @doc false
  defp validate_email_config(config) when is_map(config) do
    msg_fallback_codepage =
      Map.get(config, "msg_fallback_codepage") || Map.get(config, :msg_fallback_codepage)

    case msg_fallback_codepage do
      nil ->
        :ok

      value when is_integer(value) and value >= 0 ->
        :ok

      value when is_integer(value) ->
        {:error,
         "Field 'email.msg_fallback_codepage' must be a non-negative integer, got: #{value}"}

      value ->
        {:error,
         "Field 'email.msg_fallback_codepage' must be a non-negative integer, got: #{type_name(value)}"}
    end
  end
end
