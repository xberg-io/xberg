```elixir title="Elixir"
defmodule MyApp.WordCountProcessor do
  @behaviour Kreuzberg.Plugin.PostProcessor

  def name, do: "word_count_processor"

  def version, do: "1.0.0"

  def processing_stage, do: :late

  def initialize, do: :ok

  def shutdown, do: :ok

  def process(result, _config) do
    word_count = result.content
      |> String.split()
      |> Enum.count()

    metadata = Map.put(result.metadata || %{}, "word_count", word_count)
    %{result | metadata: metadata}
  end
end

# Register post-processor
Kreuzberg.Plugin.register_post_processor(MyApp.WordCountProcessor)

# Use with extraction
{:ok, result} = Kreuzberg.extract_file(
  "document.pdf",
  nil
)

IO.puts("Word count: #{result.metadata["word_count"]}")
```
