defmodule Kreuzberg.Plugin.Validator do
  @moduledoc """
  Behaviour module for Kreuzberg document extraction validators.

  This module defines the callback interface for implementing custom validators
  in the Kreuzberg plugin system. Validators are responsible for validating
  extraction results and ensuring data quality and consistency.

  Validators are executed in a pipeline with configurable priorities, allowing
  fine-grained control over validation order and result handling. Each validator
  can decide whether it should validate a given result based on custom logic.

  ## Validator Lifecycle

  The validator lifecycle consists of four main phases:

  1. **Initialization** - Called once when the validator is registered
     - Use this to set up resources, connect to services, etc.
     - Must return `:ok` or `{:error, reason}`

  2. **Conditional Validation** - Before validating, check if validation should run
     - Use `should_validate?/1` to conditionally apply validation logic
     - Useful for document-type-specific validators

  3. **Validation** - Perform the actual validation
     - Check result structure and content
     - Return `:ok` or `{:error, reason}` with descriptive message

  4. **Shutdown** - Called when the validator is unregistered
     - Use this to clean up resources
     - Must return `:ok`

  ## Priority System

  Validators are sorted by priority (descending) before execution. Higher priority
  values run first. This allows you to:

  - Run fast validators first (fail-fast approach)
  - Run validators with dependencies in order
  - Control the order of detailed validation passes

  Typical priority levels:
  - 100+ - Critical validators (must pass)
  - 50-100 - High priority validators
  - 1-50 - Standard validators
  - 0 or negative - Low priority (informational)

  ## Validation Results

  All validation functions return one of:
  - `:ok` - Validation passed
  - `{:error, reason}` - Validation failed with human-readable reason

  Error messages should be descriptive enough to help developers:
  - Specify what was wrong
  - Explain why it matters
  - Provide hints for fixing the issue

  ## Example Validators

  See the examples below for common validator patterns.

  ## Behaviour Callbacks

  All modules implementing this behaviour must define:

  - `name/0` - Return a unique validator identifier
  - `validate/1` - Perform validation on extraction result
  - `should_validate?/1` - Decide if validation should run
  - `priority/0` - Return validation priority (integer)
  - `initialize/0` - Set up validator resources
  - `shutdown/0` - Clean up validator resources
  - `version/0` - Return validator version string

  ## Examples

  A minimal validator that checks for empty content:

      defmodule MyApp.Validators.NonEmptyValidator do
        @behaviour Kreuzberg.Plugin.Validator

        def name, do: "non_empty_content_validator"

        def validate(result) do
          if String.length(result["content"] || "") > 0 do
            :ok
          else
            {:error, "Extraction result contains empty content"}
          end
        end

        def should_validate?(result) do
          is_map(result) and Map.has_key?(result, "content")
        end

        def priority, do: 100

        def initialize do
          :ok
        end

        def shutdown, do: :ok

        def version, do: "1.0.0"
      end

  A more complex validator that validates PDF metadata:

      defmodule MyApp.Validators.PDFMetadataValidator do
        @behaviour Kreuzberg.Plugin.Validator

        def name, do: "pdf_metadata_validator"

        def validate(result) do
          with {:ok, mime} <- validate_mime_type(result),
               {:ok, metadata} <- validate_metadata_exists(result),
               {:ok, _} <- validate_required_fields(result) do
            :ok
          end
        end

        def should_validate?(result) do
          mime_type = result["mime_type"]
          String.starts_with?(mime_type || "", "application/pdf")
        end

        def priority, do: 75

        def initialize do
          # Could initialize PDF validation library here
          :ok
        end

        def shutdown, do: :ok

        def version, do: "2.1.0"

        # Private helpers would go here
        # (implementation details omitted for brevity)
      end

  A stateful validator that tracks statistics:

      defmodule MyApp.Validators.StatisticsValidator do
        @behaviour Kreuzberg.Plugin.Validator
        use GenServer

        def name, do: "statistics_validator"

        def validate(result) do
          try do
            # Update statistics
            :ok = GenServer.call(__MODULE__, {:track, result})
            :ok
          catch
            :exit, _ ->
              {:error, "Failed to record statistics"}
          end
        end

        def should_validate?(_result), do: true

        def priority, do: 10

        def initialize do
          GenServer.start_link(__MODULE__, %{}, name: __MODULE__)
          :ok
        end

        def shutdown do
          GenServer.stop(__MODULE__)
        end

        def version, do: "1.5.0"

        # GenServer callbacks
        @impl true
        def init(state) do
          {:ok, state}
        end

        @impl true
        def handle_call({:track, result}, _from, state) do
          new_state = update_stats(state, result)
          {:reply, :ok, new_state}
        end

        defp update_stats(state, result) do
          # Track metrics based on result
          state
        end
      end

  ## Validator Registration

  Validators are registered with the Kreuzberg plugin system:

      Kreuzberg.Plugin.register_validator(MyApp.Validators.NonEmptyValidator)

  And can be unregistered when no longer needed:

      Kreuzberg.Plugin.unregister_validator("non_empty_content_validator")

  ## Validation Pipeline

  During result processing, the system:

  1. Collects all registered validators
  2. Sorts by priority (highest first)
  3. For each validator:
     a. Calls `should_validate?/1` to check applicability
     b. If true, calls `validate/1`
     c. Continues on `:ok`, may stop on error based on policy
  4. Returns combined validation result

  ## Error Handling

  When validation fails:
  - Single validation error: `{:error, "reason"}`
  - Multiple validation errors: `{:error, [{"validator_name", "reason"}, ...]}`
  - System errors: `{:error, "Validator xyz: system error"}`

  Validators should avoid raising exceptions and instead return error tuples.
  """

  @type result :: map()
  @type validation_result :: :ok | {:error, String.t()}

  @doc """
  Returns the unique identifier/name for this validator.

  The name should be a descriptive string that uniquely identifies this validator
  within the plugin system. It will be used for logging, registration, and error
  messages.

  ## Returns

  A string identifier (e.g., "pdf_content_validator", "table_format_validator").

  ## Examples

      iex> MyValidator.name()
      "content_length_validator"
  """
  @callback name() :: String.t()

  @doc """
  Validates an extraction result.

  This is the main validation function called by the plugin system. It should
  perform all necessary validation checks on the result and return either `:ok`
  or an error tuple with a descriptive message.

  The validator should not raise exceptions; use error tuples instead for
  consistent error handling in the plugin system.

  ## Parameters

    * `result` - A map containing the extraction result with keys like:
      - `"content"` - Extracted text content
      - `"mime_type"` - Document MIME type
      - `"metadata"` - Document metadata
      - `"tables"` - Extracted tables
      - And other extraction result fields

  ## Returns

    * `:ok` - If validation passes
    * `{:error, reason}` - If validation fails, with a human-readable reason

  ## Error Messages

  Error messages should be specific and helpful:
  - "Content is empty" - Good
  - "Validation failed" - Poor

  ## Examples

      iex> MyValidator.validate(%{"content" => "Hello", "mime_type" => "text/plain"})
      :ok

      iex> MyValidator.validate(%{"content" => "", "mime_type" => "text/plain"})
      {:error, "Content cannot be empty"}
  """
  @callback validate(result :: map()) :: validation_result()

  @doc """
  Determines whether this validator should validate the given result.

  This callback allows validators to conditionally apply validation logic based
  on the result content. For example, a PDF-specific validator might only
  validate results with mime_type "application/pdf".

  Returning false from this callback causes the validator to be skipped for
  that result without calling `validate/1`.

  ## Parameters

    * `result` - A map containing the extraction result to check

  ## Returns

    * `true` - This validator should validate the result
    * `false` - This validator should be skipped for this result

  ## Examples

      # Validate all results
      iex> MyValidator.should_validate?(%{"content" => "text"})
      true

      # Only validate PDFs
      iex> PDFValidator.should_validate?(%{"mime_type" => "application/pdf"})
      true

      iex> PDFValidator.should_validate?(%{"mime_type" => "text/plain"})
      false
  """
  @callback should_validate?(result :: map()) :: boolean()

  @doc """
  Returns the priority for this validator.

  Higher priority validators run first in the validation pipeline. Priority is
  used to control the order of validator execution, allowing:

  - Critical validators to fail fast
  - Validators with dependencies to run in order
  - Expensive validators to run last

  Typical values:
  - 100-200: Critical system validators
  - 50-100: High priority domain validators
  - 1-50: Standard validators
  - 0 or negative: Low priority, informational validators

  ## Returns

  An integer representing the priority (typically 0-200, but any integer is valid).

  ## Examples

      iex> MyValidator.priority()
      50

      iex> CriticalValidator.priority()
      150

      iex> InformationalValidator.priority()
      -10
  """
  @callback priority() :: integer()

  @doc """
  Initializes the validator.

  This callback is called once when the validator is registered with the plugin
  system. Use it to:

  - Set up resources (connections, file handles, etc.)
  - Initialize state
  - Validate configuration
  - Perform one-time setup

  If initialization fails, the validator will not be registered and an error
  will be returned to the caller.

  ## Returns

    * `:ok` - Initialization successful
    * `{:error, reason}` - Initialization failed with a reason

  ## Examples

      # Minimal validator with no setup
      iex> MyValidator.initialize()
      :ok

      # Validator that needs to connect to a service
      iex> ServiceValidator.initialize()
      # Attempts to connect, returns :ok or {:error, "Connection failed"}
  """
  @callback initialize() :: :ok | {:error, String.t()}

  @doc """
  Shuts down the validator.

  This callback is called when the validator is unregistered from the plugin
  system. Use it to:

  - Close resources (connections, files, etc.)
  - Clean up state
  - Stop processes

  The shutdown callback should always return `:ok` and not raise exceptions.

  ## Returns

    * `:ok` - Always returns :ok to ensure cleanup completes

  ## Examples

      # Validator with no resources
      iex> MyValidator.shutdown()
      :ok
  """
  @callback shutdown() :: :ok

  @doc """
  Returns the version of this validator.

  This should be a version string that identifies the specific implementation
  of this validator. It's useful for:

  - Debugging (knowing which version of a validator is running)
  - Logging and metrics
  - Compatibility checking

  Version format should follow semantic versioning (e.g., "1.2.3").

  ## Returns

  A version string (e.g., "1.0.0", "2.1.5-beta").

  ## Examples

      iex> MyValidator.version()
      "1.0.0"

      iex> EnhancedValidator.version()
      "2.1.0"
  """
  @callback version() :: String.t()
end
