<!-- snippet:syntax-only -->

```elixir
schema = %{
  "type" => "object",
  "properties" => %{
    "title" => %{"type" => "string"},
    "authors" => %{"type" => "array", "items" => %{"type" => "string"}},
    "date" => %{"type" => "string"}
  },
  "required" => ["title", "authors", "date"],
  "additionalProperties" => false
}

config_json =
  Jason.encode!(%{
    "structured_extraction" => %{
      "schema" => schema,
      "schema_name" => "paper_metadata",
      "strict" => true,
      "llm" => %{"model" => "openai/gpt-4o-mini"}
    }
  })

{:ok, json} = Xberg.extract_async("paper.pdf", nil, config_json)
result = Jason.decode!(json)

case result["structured_output"] do
  nil -> IO.puts("no structured output")
  output -> IO.inspect(output, label: "structured")
end
```
