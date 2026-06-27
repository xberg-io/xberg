```elixir title="Elixir"
config_json = Jason.encode!(%{
  "pdf_options" => %{
    "extract_images" => true,
    "passwords" => ["password123"],
    "extract_metadata" => true,
    "hierarchy" => %{}
  }
})

{:ok, result} = Xberg.extract_sync("encrypted.pdf", "application/pdf", config_json)
IO.puts("Title: #{inspect(result.metadata.title)}")
IO.puts("Authors: #{inspect(result.metadata.authors)}")
```
