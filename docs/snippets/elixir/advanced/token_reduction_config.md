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
if result.original_token_count do
  IO.puts("Original tokens: #{result.original_token_count}")
end
if result.reduced_token_count do
  IO.puts("Reduced tokens: #{result.reduced_token_count}")
end
```
