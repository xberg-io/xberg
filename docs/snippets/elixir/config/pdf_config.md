```elixir title="Elixir"
config_json = Jason.encode!(%{
  "pdf_options" => %{
    "extract_images" => true,
    "passwords" => ["password123"],
    "extract_metadata" => true,
    "hierarchy" => %{}
  }
})

input = %Xberg.ExtractInput{kind: :uri, uri: "encrypted.pdf", mime_type: "application/pdf"}
{:ok, output} = Xberg.extract(input, config_json)

result = List.first(output.results)
IO.puts("Title: #{inspect(result.metadata.title)}")
IO.puts("Authors: #{inspect(result.metadata.authors)}")
```
