defmodule Kreuzberg.Helpers do
  @moduledoc """
  Shared helper functions for Kreuzberg extraction modules.

  This module provides common utility functions used across multiple modules
  (Kreuzberg, BatchAPI, CacheAPI) to reduce code duplication and ensure
  consistent behavior.

  ## Features

  - Key normalization for maps (atom/string conversion)
  - Value normalization for nested structures
  - Configuration validation
  - Result struct conversion from native responses
  - Statistics key normalization for cache operations
  """

  alias Kreuzberg.{ExtractionConfig, ExtractionResult}

  @doc """
  Recursively normalize map keys to strings.

  Converts all map keys to strings, handling nested maps and lists.
  Non-map values are passed through unchanged.

  ## Parameters

    * `map` - Map or any value to normalize

  ## Returns

    * Map with string keys (for maps)
    * Value unchanged (for non-maps)

  ## Examples

      iex> Kreuzberg.Helpers.normalize_map_keys(%{a: 1, "b" => 2})
      %{"a" => 1, "b" => 2}

      iex> Kreuzberg.Helpers.normalize_map_keys(%{"nested" => %{key: "value"}})
      %{"nested" => %{"key" => "value"}}

      iex> Kreuzberg.Helpers.normalize_map_keys([1, 2, 3])
      [1, 2, 3]
  """
  @spec normalize_map_keys(any()) :: any()
  def normalize_map_keys(map) when is_map(map) do
    map
    |> Enum.reduce(%{}, fn {key, value}, acc ->
      string_key = normalize_key(key)
      normalized_value = normalize_value(value)
      Map.put(acc, string_key, normalized_value)
    end)
  end

  def normalize_map_keys(value), do: value

  @doc """
  Convert atom, string, or any value to string key.

  Handles conversion of different key types to string format,
  used for normalizing map keys from native responses.

  ## Parameters

    * `key` - Key as atom, binary, or any other type

  ## Returns

    * String representation of the key

  ## Examples

      iex> Kreuzberg.Helpers.normalize_key(:content)
      "content"

      iex> Kreuzberg.Helpers.normalize_key("content")
      "content"

      iex> Kreuzberg.Helpers.normalize_key(123)
      "123"
  """
  @spec normalize_key(atom() | binary() | any()) :: String.t()
  def normalize_key(key) when is_binary(key), do: key
  def normalize_key(key) when is_atom(key), do: Atom.to_string(key)
  def normalize_key(key), do: to_string(key)

  @doc """
  Recursively normalize nested values in maps and lists.

  Handles normalization of:
  - Nested maps: applies key normalization recursively
  - Lists: normalizes each element
  - Other values: passed through unchanged

  ## Parameters

    * `value` - Value to normalize (any type)

  ## Returns

    * Normalized value with all nested keys converted to strings

  ## Examples

      iex> Kreuzberg.Helpers.normalize_value(%{key: "value"})
      %{"key" => "value"}

      iex> Kreuzberg.Helpers.normalize_value([%{a: 1}, %{b: 2}])
      [%{"a" => 1}, %{"b" => 2}]

      iex> Kreuzberg.Helpers.normalize_value("string")
      "string"
  """
  @spec normalize_value(any()) :: any()
  def normalize_value(value) when is_map(value), do: normalize_map_keys(value)
  def normalize_value(values) when is_list(values), do: Enum.map(values, &normalize_value/1)
  def normalize_value(value), do: value

  @doc """
  Validate extraction configuration from various formats.

  Accepts nil, ExtractionConfig structs, maps, or keyword lists.
  Validates structs and passes through other formats for later processing.

  ## Parameters

    * `config` - Configuration as:
      * `nil` - No configuration (valid, returns {:ok, nil})
      * `ExtractionConfig.t()` - Struct format (validated)
      * `map()` - Key-value configuration (passed through)
      * `keyword()` - Keyword list configuration (passed through)

  ## Returns

    * `{:ok, config}` - Valid configuration
    * `{:error, reason}` - Configuration validation failed

  ## Examples

      iex> Kreuzberg.Helpers.validate_config(nil)
      {:ok, nil}

      iex> Kreuzberg.Helpers.validate_config(%Kreuzberg.ExtractionConfig{})
      {:ok, %Kreuzberg.ExtractionConfig{...}}

      iex> Kreuzberg.Helpers.validate_config(%{"extract_images" => true})
      {:ok, %{"extract_images" => true}}

      iex> Kreuzberg.Helpers.validate_config(extract_images: true)
      {:ok, [extract_images: true]}
  """
  @spec validate_config(
          nil | ExtractionConfig.t() | map() | keyword()
        ) :: {:ok, nil | ExtractionConfig.t() | map() | keyword()} | {:error, String.t()}
  def validate_config(nil), do: {:ok, nil}

  def validate_config(%ExtractionConfig{} = cfg) do
    ExtractionConfig.validate(cfg)
  end

  def validate_config(config) when is_map(config) or is_list(config) do
    # For plain maps/keyword lists, pass through without validation
    # ExtractionConfig.to_map/1 already handles string/atom key normalization
    {:ok, config}
  end

  def validate_config(_) do
    {:error, "Configuration must be a map, keyword list, or ExtractionConfig struct"}
  end

  @doc """
  Convert a native extraction response map to ExtractionResult struct.

  Takes a map from the native layer and creates a properly typed
  ExtractionResult struct, normalizing all keys to strings in the process.

  ## Parameters

    * `map` - Raw map from native extraction response

  ## Returns

    * `{:ok, ExtractionResult.t()}` - Successfully converted result struct
    * `{:error, reason}` - Conversion failed (missing required fields)

  ## Examples

      iex> native_response = %{
      ...>   "content" => "extracted text",
      ...>   "mime_type" => "application/pdf",
      ...>   metadata: %{pages: 5}
      ...> }
      iex> {:ok, result} = Kreuzberg.Helpers.into_result(native_response)
      iex> result.content
      "extracted text"

      iex> invalid_response = %{"content" => "text"}
      iex> {:error, reason} = Kreuzberg.Helpers.into_result(invalid_response)
      iex> String.contains?(reason, "mime_type")
      true
  """
  @spec into_result(map()) :: {:ok, ExtractionResult.t()} | {:error, String.t()}
  def into_result(map) when is_map(map) do
    normalized = normalize_map_keys(map)

    # Validate required fields exist
    cond do
      not Map.has_key?(normalized, "content") ->
        {:error, "Missing required field 'content' in extraction result"}

      not Map.has_key?(normalized, "mime_type") ->
        {:error, "Missing required field 'mime_type' in extraction result"}

      true ->
        {:ok,
         %ExtractionResult{
           content: normalized["content"],
           mime_type: normalized["mime_type"],
           metadata: normalized["metadata"] || %{},
           tables: normalized["tables"] || [],
           detected_languages: normalized["detected_languages"],
           chunks: normalized["chunks"],
           images: normalized["images"],
           pages: normalized["pages"]
         }}
    end
  end

  @doc """
  Normalize keys in statistics maps for cache operations.

  Converts all keys in statistics maps to strings, used specifically
  for normalizing cache statistics from the native layer.

  This is an alias for `normalize_map_keys/1` to reduce duplication.

  ## Parameters

    * `map` - Map or any value (only maps are normalized)

  ## Returns

    * Map with string keys (for maps)
    * Value unchanged (for non-maps)

  ## Examples

      iex> Kreuzberg.Helpers.normalize_stats_keys(%{total_files: 42, "total_size_mb" => 128.5})
      %{"total_files" => 42, "total_size_mb" => 128.5}

      iex> Kreuzberg.Helpers.normalize_stats_keys([1, 2, 3])
      [1, 2, 3]
  """
  @spec normalize_stats_keys(any()) :: any()
  def normalize_stats_keys(value), do: normalize_map_keys(value)

  @doc """
  Call native extraction function with configuration validation pattern.

  Handles the common pattern of:
  1. Checking if config is nil (bypass validation and call simple native function)
  2. Validating config if provided
  3. Converting config to map
  4. Calling native function with config options

  ## Parameters

    * `nil_func` - 0-arity function to call when config is nil
    * `config_func` - 1-arity function to call with config_map
    * `config` - Configuration to validate (nil, ExtractionConfig, map, or keyword list)

  ## Returns

    * `{:ok, result}` - Native function succeeded
    * `{:error, reason}` - Configuration invalid or native function failed

  ## Examples

      iex> Kreuzberg.Helpers.call_native(
      ...>   fn -> Native.extract(input, mime_type) end,
      ...>   fn cfg_map -> Native.extract_with_options(input, mime_type, cfg_map) end,
      ...>   nil
      ...> )
      {:ok, result_map}
  """
  @spec call_native(
          (-> {:ok, any()} | {:error, String.t()}),
          (map() -> {:ok, any()} | {:error, String.t()}),
          nil | ExtractionConfig.t() | map() | keyword()
        ) :: {:ok, any()} | {:error, String.t()}
  def call_native(nil_func, _config_func, nil) do
    nil_func.()
  end

  def call_native(_nil_func, config_func, config) do
    with {:ok, validated_config} <- validate_config(config),
         config_map <- ExtractionConfig.to_map(validated_config) do
      config_func.(config_map)
    else
      {:error, reason} -> {:error, "Invalid configuration: #{reason}"}
    end
  end

  @doc """
  Run a list of validators with an optional data parameter.

  Generic reducer for running validators that may or may not accept a data parameter.
  Used to reduce duplication in run_validators and run_final_validators patterns.

  ## Parameters

    * `validators` - List of validator modules to run
    * `data` - Optional data to pass to validators (defaults to nil)

  ## Returns

    * `:ok` - All validators passed
    * `{:error, reason}` - First validator that failed

  ## Examples

      iex> Kreuzberg.Helpers.run_validators([MyValidator])
      :ok

      iex> Kreuzberg.Helpers.run_validators([MyValidator], extraction_result)
      :ok
  """
  @spec run_validators([atom()], any() | nil) :: :ok | {:error, String.t()}
  def run_validators(validators, data \\ nil) do
    Enum.reduce_while(validators, :ok, fn validator_module, _acc ->
      case apply(validator_module, :validate, [data]) do
        :ok -> {:cont, :ok}
        {:error, reason} -> {:halt, {:error, "Validator #{validator_module} failed: #{reason}"}}
      end
    end)
  end
end
