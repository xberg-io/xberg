```elixir title="Elixir"
config_json = Jason.encode!(%{
  "token_reduction" => %{
    "mode" => "moderate",
    "preserve_important_words" => true
  }
})

{:ok, result} = Kreuzberg.extract_file_sync("document.pdf", "application/pdf", config_json)
IO.puts("Original tokens: #{result.token_count}")
IO.puts("Reduced content: #{result.content}")
```
