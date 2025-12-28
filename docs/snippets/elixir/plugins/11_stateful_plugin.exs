```elixir title="Elixir"
# Define a stateful post-processor plugin using an Agent
defmodule MyApp.Plugins.StatefulTextProcessor do
  @behaviour Kreuzberg.Plugin.PostProcessor

  @moduledoc """
  A stateful post-processor that maintains a count of processed documents.
  Demonstrates how to use an Agent to store state across multiple processing calls.
  """

  @impl true
  def name, do: "stateful_text_processor"

  @impl true
  def version, do: "1.0.0"

  @impl true
  def processing_stage, do: :middle

  # Start an Agent to maintain state
  @impl true
  def initialize do
    case Agent.start_link(fn -> %{count: 0, errors: 0} end, name: __MODULE__) do
      {:ok, _pid} -> :ok
      {:error, {:already_started, _}} -> :ok
      error -> error
    end
  end

  @impl true
  def shutdown do
    case Agent.stop(__MODULE__) do
      :ok -> :ok
      error -> error
    end
  end

  @impl true
  def process(result, _config) do
    # Increment the processed count
    Agent.update(__MODULE__, fn state ->
      %{state | count: state.count + 1}
    end)

    # Add metadata about processing
    case normalize_content(result.content) do
      {:ok, normalized} ->
        Map.merge(result, %{
          "content" => normalized,
          "processed_count" => get_count(),
          "processing_timestamp" => DateTime.utc_now() |> DateTime.to_iso8601()
        })

      {:error, reason} ->
        Agent.update(__MODULE__, fn state ->
          %{state | errors: state.errors + 1}
        end)

        {:error, "Failed to normalize content: #{reason}"}
    end
  end

  # Retrieve the current processing count
  defp get_count do
    Agent.get(__MODULE__, fn state -> state.count end)
  end

  # Get error count
  defp get_errors do
    Agent.get(__MODULE__, fn state -> state.errors end)
  end

  # Normalize text content
  defp normalize_content(content) when is_binary(content) do
    {:ok,
     content
     |> String.trim()
     |> String.replace(~r/\s+/, " ")}
  end

  defp normalize_content(_), do: {:error, "Content is not a string"}
end

# Register the stateful plugin
:ok = Kreuzberg.Plugin.register_post_processor(:stateful, MyApp.Plugins.StatefulTextProcessor)

# Initialize the plugin
:ok = MyApp.Plugins.StatefulTextProcessor.initialize()

# Process first document
result1 = %{"content" => "  Example   text   with   spaces  "}
processed1 = MyApp.Plugins.StatefulTextProcessor.process(result1, nil)
IO.inspect(processed1, label: "First processing")

# Process second document
result2 = %{"content" => "Another  document\nwith\tmultiple\nlines"}
processed2 = MyApp.Plugins.StatefulTextProcessor.process(result2, nil)
IO.inspect(processed2, label: "Second processing")

# The state persists across calls
IO.puts("Documents processed: #{processed2["processed_count"]}")

# Verify plugin is registered
{:ok, processors} = Kreuzberg.Plugin.list_post_processors()
IO.inspect(processors, label: "Registered processors")

# Cleanup
:ok = MyApp.Plugins.StatefulTextProcessor.shutdown()
:ok = Kreuzberg.Plugin.unregister_post_processor(:stateful)
```
