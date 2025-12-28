defmodule Kreuzberg.Plugin.PostProcessor do
  @moduledoc """
  Behaviour module for post-processor plugins in the Kreuzberg plugin system.

  Post-processor plugins allow you to modify, filter, or enrich extraction results
  after the core extraction process completes. They operate in three distinct stages
  (early, middle, late) enabling flexible composition of transformations.

  ## Overview

  Post-processors are applied to extraction results and can:

    * Transform extraction result content and structure
    * Add computed fields or metadata
    * Filter or clean extracted data
    * Validate and normalize output
    * Apply language-specific transformations
    * Enrich results with external data

  Post-processors are executed in sequence based on their stage and defined order,
  with each processor receiving the output of the previous one.

  ## Processing Stages

  Plugins operate in three stages, executed in this order:

    * `:early` - Run before other transformations, useful for initial normalization
    * `:middle` - Standard transformations and enrichment
    * `:late` - Final cleanup and validation before results are returned

  Within each stage, processors execute in the order they are registered.

  ## Configuration

  Each post-processor can be configured via the plugin configuration system:

      config :kreuzberg, :post_processors, [
        {MyApp.NormalizerProcessor, %{
          "lowercase_all" => true,
          "remove_extra_whitespace" => true
        }}
      ]

  Configuration is passed as the second argument to the `process/2` callback.

  ## Lifecycle

  Post-processors follow a standard lifecycle:

    1. `initialize/0` is called once when the plugin system starts
    2. `process/2` is called for each extraction result
    3. `shutdown/0` is called when the plugin system shuts down

  ## Example Implementation

      defmodule MyApp.TextNormalizerProcessor do
        @behaviour Kreuzberg.Plugin.PostProcessor

        @impl true
        def name() do
          "text_normalizer"
        end

        @impl true
        def version() do
          "1.0.0"
        end

        @impl true
        def processing_stage() do
          :early
        end

        @impl true
        def initialize() do
          # Load resources, establish connections, etc.
          IO.puts("Text Normalizer initialized")
          :ok
        end

        @impl true
        def shutdown() do
          # Clean up resources
          IO.puts("Text Normalizer shutting down")
          :ok
        end

        @impl true
        def process(result, config) do
          config = config || %{}

          result
          |> normalize_content(config)
          |> normalize_metadata(config)
        end

        defp normalize_content(result, config) do
          if Map.get(config, "normalize_whitespace", false) do
            content = String.trim(result.content)
            content = Regex.replace(~r/\\s+/, content, " ")
            %{result | content: content}
          else
            result
          end
        end

        defp normalize_metadata(result, config) do
          if Map.get(config, "add_processor_info", false) do
            metadata =
              Map.put(result.metadata, "processed_by", "text_normalizer")

            %{result | metadata: metadata}
          else
            result
          end
        end
      end

  ## Integration

  Post-processors are automatically discovered and loaded by the Kreuzberg plugin
  system. They should be registered in your application configuration:

      config :kreuzberg, :plugins, [
        post_processors: [
          {MyApp.TextNormalizerProcessor, %{"normalize_whitespace" => true}}
        ]
      ]

  ## Error Handling

  If a post-processor encounters an error during processing:

    * The error is logged with context
    * Processing continues with the unmodified result if `process/2` returns an error
    * Use `initialize/0` to perform validation and return errors early

  Implementations should handle errors gracefully within `process/2` and not raise
  exceptions, as this will interrupt the processing pipeline.

  ## Performance Considerations

    * Keep `process/2` implementations efficient, as they run on every extraction
    * Heavy computations should be performed during `initialize/0` if possible
    * Use `:early` stage for computationally cheap operations like normalization
    * Use `:late` stage for expensive validation or enrichment
  """

  @type result :: map()
  @type config :: map() | nil
  @type stage :: :early | :middle | :late

  @doc """
  Returns the name of this post-processor.

  The name should be a unique identifier for the processor across the system.
  Names are used for logging, configuration, and debugging purposes.

  ## Returns

  A string containing the processor name (e.g., "text_normalizer", "entity_extractor").

  ## Examples

      iex> MyApp.TextNormalizerProcessor.name()
      "text_normalizer"

      iex> MyApp.MetadataEnricherProcessor.name()
      "metadata_enricher"
  """
  @callback name() :: String.t()

  @doc """
  Processes an extraction result with optional configuration.

  This callback is invoked for each extraction result after core extraction completes.
  The processor receives the current result and its configuration, and must return
  the transformed result (which may be the same result unmodified).

  ## Parameters

    * `result` - A map containing extraction result data with fields like:
      * `:content` - Extracted text content
      * `:mime_type` - Document MIME type
      * `:metadata` - Document metadata
      * `:tables` - Extracted tables
      * `:detected_languages` - Detected language codes
      * `:chunks` - Text chunks with embeddings
      * `:images` - Extracted images
      * `:pages` - Per-page content
    * `config` - Optional configuration map passed from system configuration,
      or `nil` if no configuration was provided

  ## Returns

  The processed result map. The structure should match the input result structure
  but may contain modified or added fields.

  ## Examples

      iex> result = %{
      ...>   content: "Hello  WORLD",
      ...>   mime_type: "text/plain",
      ...>   metadata: %{}
      ...> }
      iex> config = %{"normalize_whitespace" => true}
      iex> MyApp.TextNormalizerProcessor.process(result, config)
      %{
        content: "Hello WORLD",
        mime_type: "text/plain",
        metadata: %{}
      }

      iex> result = %{content: "test", metadata: %{}}
      iex> MyApp.Processor.process(result, nil)
      %{content: "test", metadata: %{}}
  """
  @callback process(result :: result(), config :: config()) :: result()

  @doc """
  Returns the processing stage for this post-processor.

  Defines when this processor runs in the pipeline relative to other processors.
  All `:early` stage processors run before `:middle` stage processors, which run
  before `:late` stage processors.

  Within the same stage, processors execute in the order they are registered.

  ## Returns

  An atom representing the stage: `:early`, `:middle`, or `:late`.

  ## Examples

      iex> MyApp.TextNormalizerProcessor.processing_stage()
      :early

      iex> MyApp.EntityExtractorProcessor.processing_stage()
      :middle

      iex> MyApp.ValidationProcessor.processing_stage()
      :late
  """
  @callback processing_stage() :: stage()

  @doc """
  Initializes the post-processor.

  Called once when the processor is first loaded or when the plugin system starts.
  Use this callback to:

    * Load configuration files or resources
    * Establish database or service connections
    * Validate plugin requirements
    * Set up state for the processor

  The plugin system waits for this callback to complete before invoking `process/2`.
  If initialization fails, the processor is not loaded and an error is logged.

  ## Returns

    * `:ok` - Initialization succeeded, processor is ready
    * `{:error, reason}` - Initialization failed with error reason (string)

  ## Examples

      @impl true
      def initialize() do
        case load_model("model.bin") do
          {:ok, model} ->
            Agent.start_link(fn -> model end, name: __MODULE__)
            :ok
          {:error, reason} ->
            {:error, "Failed to load model: " <> reason}
        end
      end

      @impl true
      def initialize() do
        :ok
      end
  """
  @callback initialize() :: :ok | {:error, String.t()}

  @doc """
  Shuts down the post-processor.

  Called once when the processor is being unloaded or when the plugin system shuts down.
  Use this callback to:

    * Close database or service connections
    * Release allocated resources
    * Clean up temporary files
    * Flush pending data

  This callback is called for cleanup and should always succeed. Any errors raised
  or returned here are logged but do not affect system shutdown.

  ## Returns

  `:ok` - Shutdown completed (errors are logged but not propagated).

  ## Examples

      @impl true
      def shutdown() do
        Agent.stop(__MODULE__)
        :ok
      end

      @impl true
      def shutdown() do
        :ok
      end
  """
  @callback shutdown() :: :ok

  @doc """
  Returns the version of this post-processor.

  Version strings are used for:

    * Logging and debugging
    * Compatibility checks
    * Documenting which version of the processor generated results

  Use semantic versioning (e.g., "1.0.0", "2.1.3-beta").

  ## Returns

  A string containing the processor version (e.g., "1.0.0", "0.5.0").

  ## Examples

      iex> MyApp.TextNormalizerProcessor.version()
      "1.0.0"

      iex> MyApp.EntityExtractorProcessor.version()
      "2.1.3"
  """
  @callback version() :: String.t()
end
