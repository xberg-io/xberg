```elixir title="Elixir"
# Clear all plugins from the registry - useful for testing or resetting state

# Define multiple example plugins for demonstration
defmodule MyApp.Plugins.CleanupExample do
  @moduledoc """
  Example plugins to demonstrate clearing the registry.
  """

  # Simple post-processor
  defmodule TextCleaner do
    @behaviour Kreuzberg.Plugin.PostProcessor

    @impl true
    def name, do: "text_cleaner"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def processing_stage, do: :early

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def process(result, _config) do
      Map.put(result, "cleaned", true)
    end
  end

  # Simple validator
  defmodule ContentValidator do
    @behaviour Kreuzberg.Plugin.Validator

    @impl true
    def name, do: "content_validator"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def priority, do: 50

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def should_validate?(%{"content" => content}) do
      is_binary(content) and byte_size(content) > 0
    end

    def should_validate?(_), do: false

    @impl true
    def validate(%{"content" => content}) do
      if String.length(content) > 0 do
        :ok
      else
        {:error, "Content cannot be empty"}
      end
    end

    def validate(_), do: {:error, "Missing content field"}
  end

  # Simple OCR backend
  defmodule MockOCRBackend do
    @behaviour Kreuzberg.Plugin.OcrBackend

    @impl true
    def name, do: "mock_ocr"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def supported_languages, do: ["eng", "deu", "fra"]

    @impl true
    def process_image(_image_data, language) do
      if language in supported_languages() do
        {:ok, "OCR extracted text"}
      else
        {:error, "Unsupported language: #{language}"}
      end
    end

    @impl true
    def process_file(_path, language) do
      if language in supported_languages() do
        {:ok, "OCR extracted file text"}
      else
        {:error, "Unsupported language: #{language}"}
      end
    end
  end
end

# Register multiple plugins
IO.puts("=== Registering Plugins ===")
:ok = Kreuzberg.Plugin.register_post_processor(:cleaner, MyApp.Plugins.CleanupExample.TextCleaner)
:ok = Kreuzberg.Plugin.register_validator(MyApp.Plugins.CleanupExample.ContentValidator)
:ok = Kreuzberg.Plugin.register_ocr_backend(MyApp.Plugins.CleanupExample.MockOCRBackend)

# List registered plugins before clearing
{:ok, post_procs} = Kreuzberg.Plugin.list_post_processors()
{:ok, validators} = Kreuzberg.Plugin.list_validators()
{:ok, ocr_backends} = Kreuzberg.Plugin.list_ocr_backends()

IO.puts("Before clearing:")
IO.puts("  Post-processors: #{length(post_procs)}")
IO.puts("  Validators: #{length(validators)}")
IO.puts("  OCR backends: #{length(ocr_backends)}")

# Clear post-processors
IO.puts("\n=== Clearing Post-Processors ===")
:ok = Kreuzberg.Plugin.clear_post_processors()
{:ok, post_procs_after} = Kreuzberg.Plugin.list_post_processors()
IO.puts("Post-processors after clearing: #{length(post_procs_after)}")

# Validators and OCR backends should still be registered
{:ok, validators_check} = Kreuzberg.Plugin.list_validators()
{:ok, ocr_backends_check} = Kreuzberg.Plugin.list_ocr_backends()
IO.puts("Validators still registered: #{length(validators_check)}")
IO.puts("OCR backends still registered: #{length(ocr_backends_check)}")

# Clear validators
IO.puts("\n=== Clearing Validators ===")
:ok = Kreuzberg.Plugin.clear_validators()
{:ok, validators_after} = Kreuzberg.Plugin.list_validators()
IO.puts("Validators after clearing: #{length(validators_after)}")

# OCR backends should still be registered
{:ok, ocr_backends_check2} = Kreuzberg.Plugin.list_ocr_backends()
IO.puts("OCR backends still registered: #{length(ocr_backends_check2)}")

# Clear OCR backends
IO.puts("\n=== Clearing OCR Backends ===")
:ok = Kreuzberg.Plugin.clear_ocr_backends()
{:ok, ocr_backends_after} = Kreuzberg.Plugin.list_ocr_backends()
IO.puts("OCR backends after clearing: #{length(ocr_backends_after)}")

# Verify all are cleared
IO.puts("\n=== Final State (All Cleared) ===")
{:ok, final_procs} = Kreuzberg.Plugin.list_post_processors()
{:ok, final_validators} = Kreuzberg.Plugin.list_validators()
{:ok, final_backends} = Kreuzberg.Plugin.list_ocr_backends()

IO.puts("Post-processors: #{length(final_procs)}")
IO.puts("Validators: #{length(final_validators)}")
IO.puts("OCR backends: #{length(final_backends)}")

# Use case: Reset plugin state for testing
IO.puts("\n=== Common Use Case: Testing Setup/Teardown ===")

# Setup for test
Kreuzberg.Plugin.register_post_processor(:test_proc, MyApp.Plugins.CleanupExample.TextCleaner)
Kreuzberg.Plugin.register_validator(MyApp.Plugins.CleanupExample.ContentValidator)

# Run test
{:ok, test_procs} = Kreuzberg.Plugin.list_post_processors()
{:ok, test_vals} = Kreuzberg.Plugin.list_validators()
IO.puts("Test setup complete: #{length(test_procs)} processors, #{length(test_vals)} validators")

# Teardown - clear everything
Kreuzberg.Plugin.clear_post_processors()
Kreuzberg.Plugin.clear_validators()
Kreuzberg.Plugin.clear_ocr_backends()

# Verify clean state for next test
{:ok, clean_procs} = Kreuzberg.Plugin.list_post_processors()
{:ok, clean_vals} = Kreuzberg.Plugin.list_validators()
{:ok, clean_backends} = Kreuzberg.Plugin.list_ocr_backends()

IO.puts("Test teardown complete: #{length(clean_procs)} processors, #{length(clean_vals)} validators, #{length(clean_backends)} backends")
```
