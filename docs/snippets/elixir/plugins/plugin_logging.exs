```elixir title="Elixir"
alias Kreuzberg.Plugin
require Logger

# Plugin with Logging
# This example demonstrates best practices for logging in plugins.
# Proper logging helps with debugging and monitoring plugin execution.

defmodule MyApp.Plugins.LoggingProcessor do
  @behaviour Kreuzberg.Plugin.PostProcessor
  require Logger

  @impl true
  def name do
    "LoggingProcessor"
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
    try do
      Logger.debug("Starting content processing", plugin: "LoggingProcessor")

      content = result["content"] || ""
      content_size = byte_size(content)
      mime_type = result["mime_type"] || "unknown"

      Logger.info("Processing extraction result",
        mime_type: mime_type,
        content_size: content_size
      )

      # Perform processing
      processed_content = clean_content(content)

      metadata = Map.get(result, "metadata", %{})
      updated_metadata = metadata
        |> Map.put("processed_at", DateTime.utc_now())
        |> Map.put("original_size", content_size)
        |> Map.put("processed_size", byte_size(processed_content))

      Logger.debug("Processing complete",
        original_size: content_size,
        processed_size: byte_size(processed_content)
      )

      {:ok, Map.put(result, "metadata", updated_metadata)}
    rescue
      error ->
        Logger.error("Processing error in LoggingProcessor",
          error: inspect(error),
          stacktrace: __STACKTRACE__
        )
        {:error, "Processing failed: #{inspect(error)}"}
    end
  end

  defp clean_content(content) do
    content
    |> String.trim()
    |> String.replace(~r/\s+/, " ")
  end
end

# Register the logging processor
Plugin.register_post_processor(:logging_processor, MyApp.Plugins.LoggingProcessor)

# Configure logging (use :info or :debug for verbosity)
Logger.configure(level: :debug)

IO.puts("=== Plugin Logging Example ===\n")

# Example usage
result = %{
  "content" => "  Sample   document    with    irregular    spacing  ",
  "mime_type" => "application/pdf",
  "metadata" => %{"source" => "document.pdf"}
}

Logger.info("Starting extraction processing example")

case MyApp.Plugins.LoggingProcessor.process(result, %{}) do
  {:ok, processed_result} ->
    IO.puts("\nProcessing succeeded!")
    IO.inspect(processed_result, label: "Processed Result")

  {:error, reason} ->
    IO.puts("Processing failed: #{reason}")
end

# Demonstrate error handling with logging
Logger.info("Testing error handling")

invalid_result = %{
  "content" => nil,
  "mime_type" => "application/pdf"
}

case MyApp.Plugins.LoggingProcessor.process(invalid_result, %{}) do
  {:ok, _processed_result} ->
    IO.puts("Processing succeeded")

  {:error, reason} ->
    IO.puts("Processing failed as expected: #{reason}")
end
```
