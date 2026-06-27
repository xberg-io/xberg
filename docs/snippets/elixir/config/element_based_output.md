```elixir title="Element-Based Output (Elixir)"
# Configure element-based output
config = %Xberg.ExtractionConfig{
  output_format: :element_based
}

# Extract document
{:ok, result} = Xberg.extract_sync("document.pdf", config)

# Access elements
Enum.each(result.elements, fn element ->
  IO.puts("Type: #{element.element_type}")

  text = String.slice(element.text, 0, 100)
  IO.puts("Text: #{text}")

  if element.metadata.page_number do
    IO.puts("Page: #{element.metadata.page_number}")
  end

  if element.metadata.coordinates do
    coords = element.metadata.coordinates
    IO.puts("Coords: (#{coords.left}, #{coords.top}) - (#{coords.right}, #{coords.bottom})")
  end

  IO.puts("---")
end)

# Filter by element type
titles = Enum.filter(result.elements, fn e -> e.element_type == :title end)

Enum.each(titles, fn title ->
  level = Map.get(title.metadata.additional, "level", "unknown")
  IO.puts("[#{level}] #{title.text}")
end)
```
