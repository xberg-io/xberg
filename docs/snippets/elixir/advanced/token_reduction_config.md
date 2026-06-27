```elixir title="Elixir"
config_json = Jason.encode!(%{
  "token_reduction" => %{
    "mode" => "moderate",
    "preserve_markdown" => true
  }
})

{:ok, result} = Xberg.extract_sync("verbose_document.pdf", "application/pdf", config_json)

if result.original_token_count do
  IO.puts("Original tokens: #{result.original_token_count}")
end
if result.reduced_token_count do
  IO.puts("Reduced tokens: #{result.reduced_token_count}")
end
```
