```elixir title="Elixir"
# Unregister individual plugins from the registry

defmodule MyApp.Plugins.UnregisterExample do
  @moduledoc """
  Example plugins to demonstrate selective unregistration.
  """

  # Email processor post-processor
  defmodule EmailPostProcessor do
    @behaviour Kreuzberg.Plugin.PostProcessor

    @impl true
    def name, do: "email_processor"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def processing_stage, do: :middle

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def process(result, _config) do
      # Extract emails from content
      emails =
        result
        |> Map.get("content", "")
        |> String.scan(~r/[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/)
        |> Enum.map(&List.first/1)
        |> Enum.uniq()

      Map.put(result, "extracted_emails", emails)
    end
  end

  # Phone number processor post-processor
  defmodule PhonePostProcessor do
    @behaviour Kreuzberg.Plugin.PostProcessor

    @impl true
    def name, do: "phone_processor"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def processing_stage, do: :middle

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def process(result, _config) do
      # Extract phone numbers from content
      phones =
        result
        |> Map.get("content", "")
        |> String.scan(~r/\b\d{3}[-.]?\d{3}[-.]?\d{4}\b/)
        |> Enum.map(&List.first/1)
        |> Enum.uniq()

      Map.put(result, "extracted_phones", phones)
    end
  end

  # URL processor post-processor
  defmodule URLPostProcessor do
    @behaviour Kreuzberg.Plugin.PostProcessor

    @impl true
    def name, do: "url_processor"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def processing_stage, do: :middle

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def process(result, _config) do
      # Extract URLs from content
      urls =
        result
        |> Map.get("content", "")
        |> String.scan(~r/https?:\/\/\S+/)
        |> Enum.map(&List.first/1)
        |> Enum.uniq()

      Map.put(result, "extracted_urls", urls)
    end
  end

  # Strict length validator
  defmodule StrictLengthValidator do
    @behaviour Kreuzberg.Plugin.Validator

    @impl true
    def name, do: "strict_length_validator"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def priority, do: 100

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def should_validate?(%{"content" => content}) do
      is_binary(content)
    end

    def should_validate?(_), do: false

    @impl true
    def validate(%{"content" => content}) do
      min_length = 10
      max_length = 10000

      cond do
        byte_size(content) < min_length ->
          {:error, "Content too short (minimum #{min_length} bytes)"}

        byte_size(content) > max_length ->
          {:error, "Content too long (maximum #{max_length} bytes)"}

        true ->
          :ok
      end
    end

    def validate(_), do: {:error, "Missing content field"}
  end

  # Encoding validator
  defmodule EncodingValidator do
    @behaviour Kreuzberg.Plugin.Validator

    @impl true
    def name, do: "encoding_validator"

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
      is_binary(content)
    end

    def should_validate?(_), do: false

    @impl true
    def validate(%{"content" => content}) do
      if String.valid?(content) do
        :ok
      else
        {:error, "Content contains invalid UTF-8 encoding"}
      end
    end

    def validate(_), do: {:error, "Missing content field"}
  end

  # Basic OCR backend
  defmodule BasicOCR do
    @behaviour Kreuzberg.Plugin.OcrBackend

    @impl true
    def name, do: "basic_ocr"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def supported_languages, do: ["eng", "fra"]

    @impl true
    def process_image(_image_data, language) do
      if language in supported_languages() do
        {:ok, "Extracted text"}
      else
        {:error, "Unsupported language"}
      end
    end

    @impl true
    def process_file(_path, language) do
      if language in supported_languages() do
        {:ok, "Extracted file text"}
      else
        {:error, "Unsupported language"}
      end
    end
  end

  # Advanced OCR backend
  defmodule AdvancedOCR do
    @behaviour Kreuzberg.Plugin.OcrBackend

    @impl true
    def name, do: "advanced_ocr"

    @impl true
    def version, do: "2.0.0"

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def supported_languages do
      [
        "eng",
        "fra",
        "deu",
        "spa",
        "ita",
        "jpn",
        "chi",
        "chi_tra",
        "kor"
      ]
    end

    @impl true
    def process_image(_image_data, language) do
      if language in supported_languages() do
        {:ok, "Advanced extracted text"}
      else
        {:error, "Unsupported language"}
      end
    end

    @impl true
    def process_file(_path, language) do
      if language in supported_languages() do
        {:ok, "Advanced extracted file text"}
      else
        {:error, "Unsupported language"}
      end
    end
  end
end

IO.puts("=== Plugin Unregistration Example ===\n")

# Register multiple plugins of each type
IO.puts("Registering plugins...")
:ok = Kreuzberg.Plugin.register_post_processor(:emails, MyApp.Plugins.UnregisterExample.EmailPostProcessor)
:ok = Kreuzberg.Plugin.register_post_processor(:phones, MyApp.Plugins.UnregisterExample.PhonePostProcessor)
:ok = Kreuzberg.Plugin.register_post_processor(:urls, MyApp.Plugins.UnregisterExample.URLPostProcessor)

:ok = Kreuzberg.Plugin.register_validator(MyApp.Plugins.UnregisterExample.StrictLengthValidator)
:ok = Kreuzberg.Plugin.register_validator(MyApp.Plugins.UnregisterExample.EncodingValidator)

:ok = Kreuzberg.Plugin.register_ocr_backend(MyApp.Plugins.UnregisterExample.BasicOCR)
:ok = Kreuzberg.Plugin.register_ocr_backend(MyApp.Plugins.UnregisterExample.AdvancedOCR)

# List all registered plugins
{:ok, procs} = Kreuzberg.Plugin.list_post_processors()
{:ok, vals} = Kreuzberg.Plugin.list_validators()
{:ok, backends} = Kreuzberg.Plugin.list_ocr_backends()

IO.puts("Initial registration:")
IO.puts("  Post-processors: #{length(procs)} - #{inspect(Enum.map(procs, &elem(&1, 0)))}")
IO.puts("  Validators: #{length(vals)} - #{inspect(Enum.map(vals, &(elem(&1, :__struct__) || &1.name())))}")
IO.puts("  OCR backends: #{length(backends)} - #{inspect(Enum.map(backends, &(elem(&1, :__struct__) || &1.name())))}\n")

# Unregister individual post-processor
IO.puts("Unregistering post-processor ':phones'...")
:ok = Kreuzberg.Plugin.unregister_post_processor(:phones)
{:ok, procs_after1} = Kreuzberg.Plugin.list_post_processors()
IO.puts("Post-processors: #{length(procs_after1)} - #{inspect(Enum.map(procs_after1, &elem(&1, 0)))}\n")

# Unregister another post-processor
IO.puts("Unregistering post-processor ':urls'...")
:ok = Kreuzberg.Plugin.unregister_post_processor(:urls)
{:ok, procs_after2} = Kreuzberg.Plugin.list_post_processors()
IO.puts("Post-processors: #{length(procs_after2)} - #{inspect(Enum.map(procs_after2, &elem(&1, 0)))}\n")

# Unregister a validator
IO.puts("Unregistering validator 'EncodingValidator'...")
:ok = Kreuzberg.Plugin.unregister_validator(MyApp.Plugins.UnregisterExample.EncodingValidator)
{:ok, vals_after} = Kreuzberg.Plugin.list_validators()
IO.puts("Validators: #{length(vals_after)}\n")

# Unregister an OCR backend
IO.puts("Unregistering OCR backend 'BasicOCR'...")
:ok = Kreuzberg.Plugin.unregister_ocr_backend(MyApp.Plugins.UnregisterExample.BasicOCR)
{:ok, backends_after} = Kreuzberg.Plugin.list_ocr_backends()
IO.puts("OCR backends: #{length(backends_after)}\n")

# Idempotent unregistration - unregistering non-existent plugin
IO.puts("Unregistering already-unregistered plugin ':phones' (idempotent)...")
:ok = Kreuzberg.Plugin.unregister_post_processor(:phones)
IO.puts("Still returns :ok\n")

# Final state
IO.puts("=== Final State ===")
{:ok, final_procs} = Kreuzberg.Plugin.list_post_processors()
{:ok, final_vals} = Kreuzberg.Plugin.list_validators()
{:ok, final_backends} = Kreuzberg.Plugin.list_ocr_backends()

IO.puts("Remaining post-processors: #{length(final_procs)}")
IO.puts("Remaining validators: #{length(final_vals)}")
IO.puts("Remaining OCR backends: #{length(final_backends)}\n")

# Cleanup - unregister remaining plugins
IO.puts("=== Cleanup ===")
:ok = Kreuzberg.Plugin.unregister_post_processor(:emails)
:ok = Kreuzberg.Plugin.unregister_validator(MyApp.Plugins.UnregisterExample.StrictLengthValidator)
:ok = Kreuzberg.Plugin.unregister_ocr_backend(MyApp.Plugins.UnregisterExample.AdvancedOCR)

{:ok, final_clean_procs} = Kreuzberg.Plugin.list_post_processors()
{:ok, final_clean_vals} = Kreuzberg.Plugin.list_validators()
{:ok, final_clean_backends} = Kreuzberg.Plugin.list_ocr_backends()

IO.puts("After cleanup:")
IO.puts("  Post-processors: #{length(final_clean_procs)}")
IO.puts("  Validators: #{length(final_clean_vals)}")
IO.puts("  OCR backends: #{length(final_clean_backends)}")
```
