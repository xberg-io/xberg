```elixir title="Elixir"
config_json = Jason.encode!(%{
  "token_reduction" => %{
    "mode" => "moderate",
    "preserve_markdown" => true
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "verbose_document.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
# Display token reduction metrics
original = result.original_token_count || 0
reduced = result.reduced_token_count || 0

IO.puts("Original tokens: #{original}")
IO.puts("Reduced tokens: #{reduced}")

if original > 0 do
  reduction_percent = Float.round((1 - reduced / original) * 100, 2)
  IO.puts("Reduction: #{reduction_percent}%")
end

# Show sample of reduced text
if result.text do
  IO.puts("\nSample of reduced text:")
  IO.puts(String.slice(result.text, 0..200) <> "...")
end
```
