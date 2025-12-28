defmodule Kreuzberg.Plugin do
  @moduledoc """
  Public Plugin API facade for registering and managing Kreuzberg plugins.

  Note: Doctests in this module use example module names like `MyApp.TextNormalizer`
  which do not exist. When running tests with this module, use `:doctest` tag to skip
  or provide implementation modules with matching names.

  This module provides a high-level interface for plugin management, allowing applications
  to register, unregister, and list custom plugins that extend Kreuzberg's functionality.

  ## Plugin Architecture Overview

  Kreuzberg's plugin system enables extensibility through pluggable components that integrate
  with the document extraction pipeline. Plugins are registered globally and can be accessed
  by the main extraction functions to customize behavior without modifying core library code.

  The architecture follows these principles:
  - **Separation of Concerns**: Each plugin type handles a specific responsibility
  - **Composability**: Multiple plugins can work together in a pipeline
  - **Thread Safety**: All operations are coordinated through a GenServer-based registry
  - **Dynamic Registration**: Plugins can be registered/unregistered at runtime
  - **Error Isolation**: Plugin failures are caught and reported without crashing the system

  ## Plugin Types

  Kreuzberg supports three main types of plugins:

  - **PostProcessor** - Custom processing of extraction results after document analysis
    - Receives extraction result, returns modified result
    - Organized into stages: `:early`, `:middle`, `:late`
    - Can transform, filter, or enrich extracted content

  - **Validator** - Custom validation logic for extraction parameters and results
    - Runs before extraction (input validators) or after (result validators)
    - Returns `:ok` or `{:error, reason}`
    - Supports priority ordering for validation sequence

  - **OcrBackend** - Custom OCR backend implementations
    - Provides `recognize/2` for text extraction from images
    - Reports supported languages via `supported_languages/0`
    - Can be language-specific or general purpose

  ## Plugin Lifecycle

  The typical plugin lifecycle follows this flow:

  1. **Initialization** - Plugin module is defined and loaded
  2. **Registration** - Plugin is registered with the plugin registry during app startup
  3. **Usage** - Plugin is invoked during extraction with `extract_with_plugins/4`
  4. **Processing** - Plugin executes its specific function (validation, processing, OCR)
  5. **Shutdown** - Plugin is unregistered when no longer needed (optional)

  Plugins persist in the registry for the lifetime of the application unless explicitly
  unregistered. This allows them to be reused across multiple extraction operations.

  ## Error Handling in Plugins

  Each plugin type should handle errors gracefully:

  - **PostProcessors** should return `{:ok, processed_data}` on success or
    `{:error, "reason"}` on failure
  - **Validators** should return `:ok` on success or `{:error, "reason"}` on failure
  - **OCR Backends** should return `{:ok, text}` on success or handle errors appropriately

  When a plugin fails, the extraction pipeline stops and the error is propagated to the caller.
  Plugin errors are logged with the plugin module name for debugging purposes.

  ## State Management Patterns

  Plugins should follow these state management patterns:

  - **Stateless Plugins**: Most plugins should be stateless, receiving all inputs as parameters
  - **Cached State**: Use ETS or Mnesia if caching is needed across multiple calls
  - **Process-Local State**: Use Agent or GenServer within the plugin module if state is required
  - **Configuration**: Use the application environment or pass config during registration

  Example stateless post-processor:

      defmodule MyApp.TextNormalizer do
        def process(result) do
          case normalize_text(result.content) do
            {:ok, normalized_content} ->
              {:ok, %{result | content: normalized_content}}
            {:error, reason} ->
              {:error, reason}
          end
        end

        defp normalize_text(text) do
          {:ok, String.trim(text)}
        end
      end

  ## Configuration Passing

  Plugins can receive configuration in several ways:

  1. **Application Environment**: Read from `Application.get_env/2`
  2. **Process Arguments**: Pass config when registering
  3. **External State**: Store configuration in ETS or a GenServer

  When using `extract_with_plugins/4`, you can pass validators and processors explicitly:

      {:ok, result} = Kreuzberg.extract_with_plugins(
        pdf_binary,
        "application/pdf",
        config,
        validators: [MyApp.CustomValidator],
        post_processors: %{early: [MyApp.Processor1]}
      )

  ## Usage Example

      # Register a custom post-processor
      defmodule MyApp.TextCleanupPostProcessor do
        def process(text), do: String.trim(text)
      end

      :ok = Kreuzberg.Plugin.register_post_processor(:text_cleanup, MyApp.TextCleanupPostProcessor)

      # List registered post-processors
      {:ok, processors} = Kreuzberg.Plugin.list_post_processors()
      # => {:ok, [text_cleanup: MyApp.TextCleanupPostProcessor, ...]}

      # Unregister when no longer needed
      :ok = Kreuzberg.Plugin.unregister_post_processor(:text_cleanup)

  ## Quick-Start Example

  Here's a complete example of creating and using a custom plugin:

      # Define a post-processor plugin
      defmodule MyApp.Plugins.HTMLCleaner do
        def process(result) do
          # result is an ExtractionResult struct
          cleaned_content = clean_html(result.content)
          {:ok, %{result | content: cleaned_content}}
        end

        defp clean_html(content) do
          content
          |> String.replace(~r/<[^>]+>/, "")  # Remove HTML tags
          |> String.trim()
        end
      end

      # Register during application startup
      defmodule MyApp.Application do
        def start(_type, _args) do
          Kreuzberg.Plugin.register_post_processor(:html_cleaner, MyApp.Plugins.HTMLCleaner)

          children = [
            # ... your other supervisor children ...
          ]

          Supervisor.start_link(children, strategy: :one_for_one)
        end
      end

      # Use in extraction
      {:ok, result} = Kreuzberg.extract_with_plugins(
        pdf_binary,
        "application/pdf",
        nil,
        post_processors: %{early: [MyApp.Plugins.HTMLCleaner]}
      )

  ## Thread Safety

  All plugin registration and retrieval operations are thread-safe and backed by a
  GenServer-based registry. Multiple processes can safely register plugins concurrently.

  ## Best Practices

  1. **Unique Names**: Use descriptive, unique plugin names to avoid conflicts
  2. **Startup Registration**: Register plugins during application startup in your supervision tree
  3. **Error Handling**: Check return values for errors and handle appropriately
  4. **Documentation**: Document any custom plugins you create for your team
  5. **Cleanup**: Unregister plugins if they're no longer needed to free resources
  6. **Testing**: Test plugins independently before integrating with extraction pipeline
  7. **Logging**: Use Logger module to log plugin activity for debugging
  """

  @doc """
  Register a custom post-processor plugin.

  Post-processors are applied to extraction results after document analysis completes,
  allowing custom transformations, filtering, or enrichment of the extracted content.

  ## Parameters

    * `name` - Unique atom identifier for the post-processor (e.g., `:text_cleanup`, `:custom_parser`)
    * `module` - Module implementing the post-processor interface

  ## Returns

    * `:ok` - Post-processor registered successfully
    * `{:error, reason}` - Error if registration fails (e.g., already registered)

  ## Module Interface

  The post-processor module should implement the following:

    * `process(data)` - Applies custom processing to extraction result data

  ## Examples

      iex> defmodule MyApp.TextNormalizer do
      ...>   def process(text) do
      ...>     text
      ...>     |> String.trim()
      ...>     |> String.downcase()
      ...>   end
      ...> end
      iex>
      iex> Kreuzberg.Plugin.register_post_processor(:normalizer, MyApp.TextNormalizer)
      :ok
      iex>
      iex> Kreuzberg.Plugin.register_post_processor(:normalizer, MyApp.TextNormalizer)
      {:error, "Post-processor ':normalizer' is already registered"}
  """
  @spec register_post_processor(atom(), module()) :: :ok | {:error, String.t()}
  def register_post_processor(name, module) when is_atom(name) and is_atom(module) do
    Kreuzberg.Plugin.Registry.register_post_processor(name, module)
  end

  @doc """
  Unregister a post-processor plugin.

  Removes a previously registered post-processor from the plugin registry.
  Safe to call even if the plugin was never registered.

  ## Parameters

    * `name` - Atom identifier of the post-processor to unregister

  ## Returns

    * `:ok` - Post-processor unregistered successfully or was never registered
    * `{:error, reason}` - Error if unregistration fails

  ## Examples

      iex> Kreuzberg.Plugin.register_post_processor(:cleanup, MyApp.Cleanup)
      :ok
      iex> Kreuzberg.Plugin.unregister_post_processor(:cleanup)
      :ok
      iex> Kreuzberg.Plugin.unregister_post_processor(:cleanup)
      :ok
  """
  @spec unregister_post_processor(atom()) :: :ok | {:error, String.t()}
  def unregister_post_processor(name) when is_atom(name) do
    Kreuzberg.Plugin.Registry.unregister_post_processor(name)
  end

  @doc """
  Clear all registered post-processor plugins.

  Removes all post-processors from the registry, useful for testing or resetting
  plugin state.

  ## Returns

    * `:ok` - All post-processors cleared successfully
    * `{:error, reason}` - Error if clearing fails

  ## Examples

      # Example - requires Module1 and Module2 to be defined
      # iex> Kreuzberg.Plugin.register_post_processor(:p1, Module1)
      # :ok
      # iex> Kreuzberg.Plugin.register_post_processor(:p2, Module2)
      # :ok
      # iex> Kreuzberg.Plugin.clear_post_processors()
      # :ok
      # iex> {:ok, list} = Kreuzberg.Plugin.list_post_processors()
      # iex> list
      # []
  """
  @spec clear_post_processors() :: :ok | {:error, String.t()}
  def clear_post_processors do
    Kreuzberg.Plugin.Registry.clear_post_processors()
  end

  @doc """
  List all registered post-processor plugins.

  Returns a list of all post-processors currently registered in the plugin registry,
  as keyword pairs of names to modules.

  ## Returns

    * `{:ok, processors}` - List of {name, module} tuples for all registered post-processors
    * `{:error, reason}` - Error if retrieval fails

  ## Examples

      iex> Kreuzberg.Plugin.register_post_processor(:cleanup, MyApp.Cleanup)
      :ok
      iex> Kreuzberg.Plugin.register_post_processor(:validate, MyApp.Validate)
      :ok
      iex> {:ok, list} = Kreuzberg.Plugin.list_post_processors()
      iex> Enum.count(list)
      2
      iex> Enum.find(list, fn {name, _mod} -> name == :cleanup end)
      {:cleanup, MyApp.Cleanup}
  """
  @spec list_post_processors() :: {:ok, [{atom(), module()}]} | {:error, String.t()}
  def list_post_processors do
    processors = Kreuzberg.Plugin.Registry.list_post_processors()
    # Extract just the name and module from the metadata
    result = Enum.map(processors, fn {name, metadata} -> {name, metadata.module} end)
    {:ok, result}
  end

  @doc """
  Register a custom validator plugin.

  Validators provide custom validation logic for extraction parameters, configuration,
  or results. They can be used to enforce domain-specific constraints before or after
  extraction.

  ## Parameters

    * `module` - Module implementing the validator interface

  ## Returns

    * `:ok` - Validator registered successfully
    * `{:error, reason}` - Error if registration fails (e.g., already registered)

  ## Module Interface

  The validator module should implement the following:

    * `validate(data)` - Validates data and returns `:ok` or `{:error, reason}`

  ## Examples

      iex> defmodule MyApp.StrictValidator do
      ...>   def validate(data) do
      ...>     if data && data != "", do: :ok, else: {:error, "Data is empty"}
      ...>   end
      ...> end
      iex>
      iex> Kreuzberg.Plugin.register_validator(MyApp.StrictValidator)
      :ok
  """
  @spec register_validator(module()) :: :ok | {:error, String.t()}
  def register_validator(module) when is_atom(module) do
    Kreuzberg.Plugin.Registry.register_validator(module)
  end

  @doc """
  Unregister a validator plugin.

  Removes a previously registered validator from the plugin registry.
  Safe to call even if the validator was never registered.

  ## Parameters

    * `module` - Module identifier of the validator to unregister

  ## Returns

    * `:ok` - Validator unregistered successfully or was never registered
    * `{:error, reason}` - Error if unregistration fails

  ## Examples

      iex> Kreuzberg.Plugin.register_validator(MyApp.Validator)
      :ok
      iex> Kreuzberg.Plugin.unregister_validator(MyApp.Validator)
      :ok
      iex> Kreuzberg.Plugin.unregister_validator(MyApp.Validator)
      :ok
  """
  @spec unregister_validator(module()) :: :ok | {:error, String.t()}
  def unregister_validator(module) when is_atom(module) do
    # Convert module to name for unregistration
    name = Kreuzberg.Plugin.Registry.module_to_name(module)
    Kreuzberg.Plugin.Registry.unregister_validator(name)
  end

  @doc """
  Clear all registered validator plugins.

  Removes all validators from the registry, useful for testing or resetting
  plugin state.

  ## Returns

    * `:ok` - All validators cleared successfully
    * `{:error, reason}` - Error if clearing fails

  ## Examples

      # Example - requires Module1 and Module2 to be defined
      # iex> Kreuzberg.Plugin.register_validator(Module1)
      # :ok
      # iex> Kreuzberg.Plugin.register_validator(Module2)
      # :ok
      # iex> Kreuzberg.Plugin.clear_validators()
      # :ok
      # iex> {:ok, list} = Kreuzberg.Plugin.list_validators()
      # iex> list
      # []
  """
  @spec clear_validators() :: :ok | {:error, String.t()}
  def clear_validators do
    Kreuzberg.Plugin.Registry.clear_validators()
  end

  @doc """
  List all registered validator plugins.

  Returns a list of all validators currently registered in the plugin registry,
  as a list of modules.

  ## Returns

    * `{:ok, validators}` - List of validator modules
    * `{:error, reason}` - Error if retrieval fails

  ## Examples

      iex> Kreuzberg.Plugin.register_validator(MyApp.ValidatorA)
      :ok
      iex> Kreuzberg.Plugin.register_validator(MyApp.ValidatorB)
      :ok
      iex> {:ok, list} = Kreuzberg.Plugin.list_validators()
      iex> Enum.count(list)
      2
      iex> Enum.member?(list, MyApp.ValidatorA)
      true
  """
  @spec list_validators() :: {:ok, [module()]} | {:error, String.t()}
  def list_validators do
    validators = Kreuzberg.Plugin.Registry.list_validators()
    # Extract just the modules
    result = Enum.map(validators, fn {_name, metadata} -> metadata.module end)
    {:ok, result}
  end

  @doc """
  Register a custom OCR backend plugin.

  OCR backends provide alternative implementations for optical character recognition,
  allowing integration of custom or specialized OCR engines beyond the built-in
  Tesseract, EasyOCR, and PaddleOCR backends.

  ## Parameters

    * `module` - Module implementing the OCR backend interface

  ## Returns

    * `:ok` - OCR backend registered successfully
    * `{:error, reason}` - Error if registration fails (e.g., already registered)

  ## Module Interface

  The OCR backend module should implement the following:

    * `recognize(image_data, language)` - Performs OCR on image data and returns extracted text
    * `supported_languages()` - Returns list of supported language codes

  ## Examples

      iex> defmodule MyApp.CustomOCRBackend do
      ...>   def recognize(image_data, language) do
      ...>     # Custom OCR logic
      ...>     {:ok, "Extracted text"}
      ...>   end
      ...>
      ...>   def supported_languages do
      ...>     ["en", "de", "fr"]
      ...>   end
      ...> end
      iex>
      iex> Kreuzberg.Plugin.register_ocr_backend(MyApp.CustomOCRBackend)
      :ok
  """
  @spec register_ocr_backend(module()) :: :ok | {:error, String.t()}
  def register_ocr_backend(module) when is_atom(module) do
    Kreuzberg.Plugin.Registry.register_ocr_backend(module)
  end

  @doc """
  Unregister an OCR backend plugin.

  Removes a previously registered OCR backend from the plugin registry.
  Safe to call even if the backend was never registered.

  ## Parameters

    * `module` - Module identifier of the OCR backend to unregister

  ## Returns

    * `:ok` - OCR backend unregistered successfully or was never registered
    * `{:error, reason}` - Error if unregistration fails

  ## Examples

      iex> Kreuzberg.Plugin.register_ocr_backend(MyApp.OCRBackend)
      :ok
      iex> Kreuzberg.Plugin.unregister_ocr_backend(MyApp.OCRBackend)
      :ok
      iex> Kreuzberg.Plugin.unregister_ocr_backend(MyApp.OCRBackend)
      :ok
  """
  @spec unregister_ocr_backend(module()) :: :ok | {:error, String.t()}
  def unregister_ocr_backend(module) when is_atom(module) do
    # Convert module to name for unregistration
    name = Kreuzberg.Plugin.Registry.module_to_name(module)
    Kreuzberg.Plugin.Registry.unregister_ocr_backend(name)
  end

  @doc """
  Clear all registered OCR backend plugins.

  Removes all OCR backends from the registry, useful for testing or resetting
  plugin state.

  ## Returns

    * `:ok` - All OCR backends cleared successfully
    * `{:error, reason}` - Error if clearing fails

  ## Examples

      # Example - requires MyApp.BackendA and MyApp.BackendB to be defined
      # iex> Kreuzberg.Plugin.register_ocr_backend(MyApp.BackendA)
      # :ok
      # iex> Kreuzberg.Plugin.register_ocr_backend(MyApp.BackendB)
      # :ok
      # iex> Kreuzberg.Plugin.clear_ocr_backends()
      # :ok
      # iex> {:ok, list} = Kreuzberg.Plugin.list_ocr_backends()
      # iex> list
      # []
  """
  @spec clear_ocr_backends() :: :ok | {:error, String.t()}
  def clear_ocr_backends do
    Kreuzberg.Plugin.Registry.clear_ocr_backends()
  end

  @doc """
  List all registered OCR backend plugins.

  Returns a list of all OCR backends currently registered in the plugin registry,
  as a list of modules.

  ## Returns

    * `{:ok, backends}` - List of OCR backend modules
    * `{:error, reason}` - Error if retrieval fails

  ## Examples

      iex> Kreuzberg.Plugin.register_ocr_backend(MyApp.BackendA)
      :ok
      iex> Kreuzberg.Plugin.register_ocr_backend(MyApp.BackendB)
      :ok
      iex> {:ok, list} = Kreuzberg.Plugin.list_ocr_backends()
      iex> Enum.count(list)
      2
      iex> Enum.member?(list, MyApp.BackendA)
      true
  """
  @spec list_ocr_backends() :: {:ok, [module()]} | {:error, String.t()}
  def list_ocr_backends do
    backends = Kreuzberg.Plugin.Registry.list_ocr_backends()
    # Extract just the modules
    result = Enum.map(backends, fn {_name, metadata} -> metadata.module end)
    {:ok, result}
  end
end
