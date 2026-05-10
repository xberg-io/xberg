```elixir title="Elixir"
config_json = Jason.encode!(%{
  "output_format" => "Html",
  "html_output" => %{
    "theme" => "GitHub"
  }
})

{:ok, result} = Kreuzberg.extract_file_sync("document.pdf", "application/pdf", config_json)
IO.puts(result.content)
```
