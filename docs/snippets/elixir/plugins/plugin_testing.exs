```elixir title="Elixir"
alias Kreuzberg.Plugin

# Plugin Testing Example
# This demonstrates how to test custom plugins with various scenarios.

defmodule MyApp.Plugins.CustomJsonExtractor do
  @behaviour Kreuzberg.Plugin.PostProcessor
  require Logger

  @impl true
  def name do
    "CustomJsonExtractor"
  end

  @impl true
  def processing_stage do
    :post
  end

  @impl true
  def version do
    "1.0.0"
  end

  @impl true
  def initialize do
    :ok
  end

  @impl true
  def shutdown do
    :ok
  end

  @impl true
  def process(result, _options) do
    mime_type = result["mime_type"] || ""

    if mime_type == "application/json" do
      Logger.debug("Processing JSON content")
      {:ok, Map.put(result, "is_json", true)}
    else
      Logger.debug("Not JSON content, skipping")
      {:ok, result}
    end
  end
end

# Test setup
defmodule MyApp.Plugins.Test do
  require Logger

  def run_tests do
    IO.puts("\n=== Running Plugin Tests ===\n")

    test_json_processing()
    test_non_json_processing()
    test_empty_content()
    test_missing_mime_type()

    IO.puts("\n=== All Tests Completed ===\n")
  end

  defp test_json_processing do
    IO.puts("Test 1: JSON Content Processing")

    result = %{
      "content" => ~s({"message": "Hello, world!"}),
      "mime_type" => "application/json",
      "metadata" => %{}
    }

    case MyApp.Plugins.CustomJsonExtractor.process(result, %{}) do
      {:ok, processed} ->
        if processed["is_json"] == true do
          IO.puts("  PASS: JSON content marked correctly\n")
        else
          IO.puts("  FAIL: JSON flag not set\n")
        end

      {:error, reason} ->
        IO.puts("  FAIL: #{reason}\n")
    end
  end

  defp test_non_json_processing do
    IO.puts("Test 2: Non-JSON Content Processing")

    result = %{
      "content" => "Plain text content",
      "mime_type" => "text/plain",
      "metadata" => %{}
    }

    case MyApp.Plugins.CustomJsonExtractor.process(result, %{}) do
      {:ok, processed} ->
        if not Map.has_key?(processed, "is_json") or !processed["is_json"] do
          IO.puts("  PASS: Non-JSON content not marked\n")
        else
          IO.puts("  FAIL: Non-JSON content incorrectly marked as JSON\n")
        end

      {:error, reason} ->
        IO.puts("  FAIL: #{reason}\n")
    end
  end

  defp test_empty_content do
    IO.puts("Test 3: Empty Content")

    result = %{
      "content" => "",
      "mime_type" => "application/json",
      "metadata" => %{}
    }

    case MyApp.Plugins.CustomJsonExtractor.process(result, %{}) do
      {:ok, _processed} ->
        IO.puts("  PASS: Empty content handled gracefully\n")

      {:error, reason} ->
        IO.puts("  FAIL: #{reason}\n")
    end
  end

  defp test_missing_mime_type do
    IO.puts("Test 4: Missing MIME Type")

    result = %{
      "content" => "Some content",
      "metadata" => %{}
    }

    case MyApp.Plugins.CustomJsonExtractor.process(result, %{}) do
      {:ok, _processed} ->
        IO.puts("  PASS: Missing MIME type handled gracefully\n")

      {:error, reason} ->
        IO.puts("  FAIL: #{reason}\n")
    end
  end
end

# Register the custom plugin
Plugin.register_post_processor(:custom_json_extractor, MyApp.Plugins.CustomJsonExtractor)

# Run the test suite
MyApp.Plugins.Test.run_tests()

# List all registered post-processors
{:ok, processors} = Plugin.list_post_processors()
IO.puts("Registered Post-Processors:")
Enum.each(processors, fn {name, module} ->
  IO.puts("  - #{name}: #{module}")
end)
```
