```elixir title="Elixir"
config_json = Jason.encode!(%{
  "output_format" => "Html",
  "html_output" => %{
    "theme" => "GitHub"
  }
})

{:ok, result} = Xberg.extract_sync("document.pdf", "application/pdf", config_json)
IO.puts(result.content)
```
