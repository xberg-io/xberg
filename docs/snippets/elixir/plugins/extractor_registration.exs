```elixir title="Elixir"
defmodule MyApp.MetadataEnricher do
  @behaviour Kreuzberg.Plugin.PostProcessor

  def process(result, _config) do
    enriched_metadata = result.metadata || %{}
    enriched_metadata = Map.put(enriched_metadata, "enriched_at", DateTime.utc_now())
    {:ok, %{result | metadata: enriched_metadata}}
  end

  def initialize, do: :ok
  def shutdown, do: :ok
  def version, do: "1.0.0"
end

defmodule MyApp.LinkExtractor do
  @behaviour Kreuzberg.Plugin.PostProcessor

  def process(result, _config) do
    links = extract_links(result.content)
    metadata = result.metadata || %{}
    metadata = Map.put(metadata, "links", links)
    {:ok, %{result | metadata: metadata}}
  end

  defp extract_links(content) do
    Regex.scan(~r/https?:\/\/\S+/, content)
    |> Enum.map(&List.first/1)
    |> Enum.uniq()
  end

  def initialize, do: :ok
  def shutdown, do: :ok
  def version, do: "1.0.0"
end

defmodule MyApp.QualityValidator do
  @behaviour Kreuzberg.Plugin.Validator

  def validate(result) do
    if String.length(result.content) > 100 do
      :ok
    else
      {:error, "Content quality too low"}
    end
  end

  def should_validate?(_result), do: true
  def priority, do: 5
  def initialize, do: :ok
  def shutdown, do: :ok
  def version, do: "1.0.0"
end

# Register multiple plugins
Kreuzberg.Plugin.register_post_processor(:metadata_enricher, MyApp.MetadataEnricher)
Kreuzberg.Plugin.register_post_processor(:link_extractor, MyApp.LinkExtractor)
Kreuzberg.Plugin.register_validator(MyApp.QualityValidator)

IO.puts("Plugins registered successfully")
```
