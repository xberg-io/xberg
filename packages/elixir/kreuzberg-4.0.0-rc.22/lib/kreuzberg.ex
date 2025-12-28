defmodule Kreuzberg do
  @moduledoc """
  High-performance document extraction for Elixir.

  ## Examples

      # Extract from binary with MIME type
      {:ok, result} = Kreuzberg.extract(pdf_binary, "application/pdf")

      # With configuration
      config = %Kreuzberg.ExtractionConfig{extract_images: true}
      {:ok, result} = Kreuzberg.extract(pdf_binary, "application/pdf", config)

      # Bang variant
      result = Kreuzberg.extract!(pdf_binary, "application/pdf")
  """

  alias Kreuzberg.{Error, ExtractionConfig, ExtractionResult, Native, Helpers}

  # Delegate batch operations to BatchAPI
  defdelegate batch_extract_files(paths, mime_type \\ nil, config \\ nil), to: Kreuzberg.BatchAPI
  defdelegate batch_extract_files!(paths, mime_type \\ nil, config \\ nil), to: Kreuzberg.BatchAPI
  defdelegate batch_extract_bytes(data_list, mime_types, config \\ nil), to: Kreuzberg.BatchAPI
  defdelegate batch_extract_bytes!(data_list, mime_types, config \\ nil), to: Kreuzberg.BatchAPI

  # Delegate async operations to AsyncAPI
  defdelegate extract_async(input, mime_type, config \\ nil), to: Kreuzberg.AsyncAPI
  defdelegate extract_file_async(path, mime_type \\ nil, config \\ nil), to: Kreuzberg.AsyncAPI

  defdelegate batch_extract_files_async(paths, mime_type \\ nil, config \\ nil),
    to: Kreuzberg.AsyncAPI

  defdelegate batch_extract_bytes_async(data_list, mime_types, config \\ nil),
    to: Kreuzberg.AsyncAPI

  # Delegate utility functions to UtilityAPI
  defdelegate detect_mime_type(data), to: Kreuzberg.UtilityAPI
  defdelegate detect_mime_type_from_path(path), to: Kreuzberg.UtilityAPI
  defdelegate validate_mime_type(mime_type), to: Kreuzberg.UtilityAPI
  defdelegate get_extensions_for_mime(mime_type), to: Kreuzberg.UtilityAPI
  defdelegate list_embedding_presets(), to: Kreuzberg.UtilityAPI
  defdelegate get_embedding_preset(name), to: Kreuzberg.UtilityAPI
  defdelegate classify_error(reason), to: Kreuzberg.UtilityAPI
  defdelegate get_error_details(), to: Kreuzberg.UtilityAPI

  # Delegate cache operations to CacheAPI
  defdelegate cache_stats(), to: Kreuzberg.CacheAPI
  defdelegate cache_stats!(), to: Kreuzberg.CacheAPI
  defdelegate clear_cache(), to: Kreuzberg.CacheAPI
  defdelegate clear_cache!(), to: Kreuzberg.CacheAPI

  # Delegate validators to Validators module
  defdelegate validate_chunking_params(params), to: Kreuzberg.Validators
  defdelegate validate_language_code(code), to: Kreuzberg.Validators
  defdelegate validate_dpi(dpi), to: Kreuzberg.Validators
  defdelegate validate_confidence(confidence), to: Kreuzberg.Validators
  defdelegate validate_ocr_backend(backend), to: Kreuzberg.Validators
  defdelegate validate_binarization_method(method), to: Kreuzberg.Validators
  defdelegate validate_tesseract_psm(psm), to: Kreuzberg.Validators
  defdelegate validate_tesseract_oem(oem), to: Kreuzberg.Validators

  @doc """
  Extract content from binary document data.

  Performs document extraction on binary input with support for various file formats.
  Returns extracted content including text, metadata, tables, images, and more.
  If no configuration is provided, uses default extraction settings.

  ## Parameters

    * `input` - Binary document data to extract from
    * `mime_type` - MIME type of the document (e.g., "application/pdf", "text/plain")
    * `config` - ExtractionConfig struct, map, keyword list, or nil (optional, defaults to nil)

  ## Returns

    * `{:ok, ExtractionResult.t()}` - Successfully extracted content with metadata
    * `{:error, reason}` - Extraction failed with error message

  ## Examples

      # Extract from binary with MIME type
      {:ok, result} = Kreuzberg.extract(pdf_binary, "application/pdf")
      result.content

      # Extract with configuration
      config = %Kreuzberg.ExtractionConfig{ocr: %{"enabled" => true}}
      {:ok, result} = Kreuzberg.extract(pdf_binary, "application/pdf", config)

      # With keyword list configuration
      {:ok, result} = Kreuzberg.extract(
        pdf_binary,
        "application/pdf",
        ocr: %{"enabled" => true}
      )
  """
  @spec extract(binary(), String.t(), ExtractionConfig.t() | map() | keyword() | nil) ::
          {:ok, ExtractionResult.t()} | {:error, String.t()}
  def extract(input, mime_type, config \\ nil) when is_binary(input) and is_binary(mime_type) do
    case call_native(input, mime_type, config) do
      {:ok, result_map} ->
        case into_result(result_map) do
          {:ok, result} -> {:ok, result}
          {:error, reason} -> {:error, "Failed to convert extraction result: #{reason}"}
        end

      {:error, _reason} = err ->
        err
    end
  end

  @doc "Extract content, raising on error"
  @spec extract!(binary(), String.t(), ExtractionConfig.t() | map() | keyword() | nil) ::
          ExtractionResult.t()
  def extract!(input, mime_type, config \\ nil) do
    case extract(input, mime_type, config) do
      {:ok, result} ->
        result

      {:error, reason} ->
        raise Error, message: reason, reason: Kreuzberg.UtilityAPI.classify_error(reason)
    end
  end

  @doc """
  Extract content from a file at the given path.

  Accepts a file path and optional MIME type, returning extracted content.
  If no MIME type is provided, the library will attempt to detect it from the file.

  ## Parameters

    * `path` - File path (String or Path.t())
    * `mime_type` - MIME type of the file (optional, defaults to nil for auto-detection)
    * `config` - ExtractionConfig struct or map with extraction options (optional)

  ## Returns

    * `{:ok, ExtractionResult.t()}` - Successfully extracted content
    * `{:error, reason}` - Extraction failed with error message

  ## Examples

      # Extract with explicit MIME type
      {:ok, result} = Kreuzberg.extract_file("document.pdf", "application/pdf")
      result.content

      # Extract with auto-detection
      {:ok, result} = Kreuzberg.extract_file("document.pdf")

      # With configuration
      config = %Kreuzberg.ExtractionConfig{extract_images: true}
      {:ok, result} = Kreuzberg.extract_file("document.pdf", "application/pdf", config)

      # With keyword list configuration
      {:ok, result} = Kreuzberg.extract_file(
        "document.pdf",
        "application/pdf",
        ocr: %{"enabled" => true}
      )
  """
  @spec extract_file(
          String.t() | Path.t(),
          String.t() | nil,
          ExtractionConfig.t() | map() | keyword() | nil
        ) ::
          {:ok, ExtractionResult.t()} | {:error, String.t()}
  def extract_file(path, mime_type \\ nil, config \\ nil)
      when (is_binary(path) or is_struct(path)) and
             (is_nil(mime_type) or is_binary(mime_type)) do
    path_string = to_string(path)

    case call_native_file(path_string, mime_type, config) do
      {:ok, result_map} ->
        case into_result(result_map) do
          {:ok, result} -> {:ok, result}
          {:error, reason} -> {:error, "Failed to convert extraction result: #{reason}"}
        end

      {:error, _reason} = err ->
        err
    end
  end

  @doc """
  Extract content from a file, raising on error.

  Same as `extract_file/3` but raises a `Kreuzberg.Error` exception if extraction fails.

  ## Parameters

    * `path` - File path (String or Path.t())
    * `mime_type` - MIME type of the file (optional, defaults to nil for auto-detection)
    * `config` - ExtractionConfig struct or map with extraction options (optional)

  ## Returns

    * `ExtractionResult.t()` - Successfully extracted content

  ## Raises

    * `Kreuzberg.Error` - If extraction fails

  ## Examples

      # Extract with explicit MIME type, raising on error
      result = Kreuzberg.extract_file!("document.pdf", "application/pdf")
      result.content

      # Extract with auto-detection, raising on error
      result = Kreuzberg.extract_file!("document.pdf")
      result.content

      # With configuration
      config = %Kreuzberg.ExtractionConfig{ocr: %{"enabled" => true}}
      result = Kreuzberg.extract_file!("document.pdf", "application/pdf", config)
  """
  @spec extract_file!(
          String.t() | Path.t(),
          String.t() | nil,
          ExtractionConfig.t() | map() | keyword() | nil
        ) :: ExtractionResult.t()
  def extract_file!(path, mime_type \\ nil, config \\ nil) do
    case extract_file(path, mime_type, config) do
      {:ok, result} ->
        result

      {:error, reason} ->
        raise Error, message: reason, reason: Kreuzberg.UtilityAPI.classify_error(reason)
    end
  end

  @doc """
  Extract content with plugin processing support.

  Performs document extraction with additional processing through registered plugins.
  Applies validators before extraction, post-processors by stage (early, middle, late) after extraction,
  and optional final validators to the result.

  Plugins are retrieved from the Plugin.Registry if not explicitly provided in plugin_opts.

  ## Parameters

    * `input` - Binary document data to extract from
    * `mime_type` - MIME type of the document (e.g., "application/pdf")
    * `config` - ExtractionConfig struct, map, keyword list, or nil for extraction (optional)
    * `plugin_opts` - Keyword list of plugin options (optional):
      * `:validators` - List of validator modules to run before extraction
      * `:post_processors` - Map of stage atoms to lists of post-processor modules
        * `:early` - Applied first to extraction result
        * `:middle` - Applied after early processors
        * `:late` - Applied last before final validators
      * `:final_validators` - List of validator modules to run after post-processing

  ## Returns

    * `{:ok, ExtractionResult.t()}` - Successfully extracted and processed content
    * `{:error, reason}` - Extraction or processing failed with error message

  ## Plugin Processing Flow

  1. **Validators** - If specified, run input validators to check extraction preconditions
  2. **Extraction** - Call `extract/3` to get initial result
  3. **Post-Processors** - Apply by stage in order (early → middle → late)
     - Each processor receives the extraction result or output from previous processor
     - Processors should return modified result or data
  4. **Final Validators** - If specified, validate the processed result
  5. **Return** - Return enhanced extraction result

  ## Examples

      # Extract with registered validators and post-processors
      {:ok, result} = Kreuzberg.extract_with_plugins(
        pdf_binary,
        "application/pdf",
        nil,
        validators: [MyApp.InputValidator],
        post_processors: %{
          early: [MyApp.EarlyProcessor],
          middle: [MyApp.MiddleProcessor],
          late: [MyApp.FinalProcessor]
        },
        final_validators: [MyApp.ResultValidator]
      )

      # Extract with only post-processors
      {:ok, result} = Kreuzberg.extract_with_plugins(
        pdf_binary,
        "application/pdf",
        %{use_cache: true},
        post_processors: %{
          early: [MyApp.Processor1, MyApp.Processor2]
        }
      )

      # Extract with configuration and validators only
      config = %Kreuzberg.ExtractionConfig{ocr: %{"enabled" => true}}
      {:ok, result} = Kreuzberg.extract_with_plugins(
        pdf_binary,
        "application/pdf",
        config,
        validators: [MyApp.Validator]
      )

      # Extract with no plugins (standard extraction)
      {:ok, result} = Kreuzberg.extract_with_plugins(pdf_binary, "application/pdf")
  """
  @spec extract_with_plugins(
          binary(),
          String.t(),
          ExtractionConfig.t() | map() | keyword() | nil,
          keyword()
        ) :: {:ok, ExtractionResult.t()} | {:error, String.t()}
  def extract_with_plugins(input, mime_type, config \\ nil, plugin_opts \\ [])
      when is_binary(input) and is_binary(mime_type) and is_list(plugin_opts) do
    with :ok <- run_validators(Keyword.get(plugin_opts, :validators, [])),
         {:ok, result} <- extract(input, mime_type, config),
         {:ok, processed_result} <-
           apply_post_processors(result, Keyword.get(plugin_opts, :post_processors, %{})),
         :ok <- run_final_validators(Keyword.get(plugin_opts, :final_validators, []), processed_result) do
      {:ok, processed_result}
    else
      {:error, reason} -> {:error, reason}
    end
  end

  # Private

  defp run_validators(validators) do
    Enum.reduce_while(validators, :ok, fn validator_module, _acc ->
      try do
        case apply(validator_module, :validate, [nil]) do
          :ok -> {:cont, :ok}
          {:error, reason} -> {:halt, {:error, "Validator #{validator_module} failed: #{reason}"}}
        end
      rescue
        exception ->
          error_msg =
            "Plugin #{inspect(validator_module)} raised exception: #{inspect(exception)}"

          {:halt, {:error, error_msg}}
      end
    end)
  end

  defp apply_post_processors(result, post_processors) when is_map(post_processors) do
    stages = [:early, :middle, :late]

    Enum.reduce_while(stages, {:ok, result}, fn stage, {:ok, current_result} ->
      processors = Map.get(post_processors, stage, [])

      case apply_processors_for_stage(current_result, processors) do
        {:ok, processed} -> {:cont, {:ok, processed}}
        {:error, reason} -> {:halt, {:error, reason}}
      end
    end)
  end

  defp apply_post_processors(result, _), do: {:ok, result}

  defp apply_processors_for_stage(result, processors) do
    Enum.reduce_while(processors, {:ok, result}, fn processor_module, {:ok, current_data} ->
      try do
        case apply(processor_module, :process, [current_data, nil]) do
          {:ok, processed} -> {:cont, {:ok, processed}}
          processed when is_struct(processed, ExtractionResult) -> {:cont, {:ok, processed}}
          {:error, reason} -> {:halt, {:error, "PostProcessor #{processor_module} failed: #{reason}"}}
        end
      rescue
        exception ->
          error_msg =
            "Plugin #{inspect(processor_module)} raised exception: #{inspect(exception)}"

          {:halt, {:error, error_msg}}
      end
    end)
  end

  defp run_final_validators(validators, result) do
    Enum.reduce_while(validators, :ok, fn validator_module, _acc ->
      try do
        case apply(validator_module, :validate, [result]) do
          :ok -> {:cont, :ok}
          {:error, reason} ->
            {:halt, {:error, "Final validator #{validator_module} failed: #{reason}"}}
        end
      rescue
        exception ->
          error_msg =
            "Plugin #{inspect(validator_module)} raised exception: #{inspect(exception)}"

          {:halt, {:error, error_msg}}
      end
    end)
  end

  defp call_native(input, mime_type, nil) do
    Native.extract(input, mime_type)
  end

  defp call_native(input, mime_type, config) do
    with {:ok, validated_config} <- Helpers.validate_config(config),
         config_map <- ExtractionConfig.to_map(validated_config) do
      Native.extract_with_options(input, mime_type, config_map)
    else
      {:error, reason} -> {:error, "Invalid configuration: #{reason}"}
    end
  end

  defp call_native_file(path, mime_type, nil) do
    Native.extract_file(path, mime_type)
  end

  defp call_native_file(path, mime_type, config) do
    with {:ok, validated_config} <- Helpers.validate_config(config),
         config_map <- ExtractionConfig.to_map(validated_config) do
      Native.extract_file_with_options(path, mime_type, config_map)
    else
      {:error, reason} -> {:error, "Invalid configuration: #{reason}"}
    end
  end

  defp into_result(map) when is_map(map) do
    Helpers.into_result(map)
  end
end
