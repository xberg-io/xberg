```elixir title="Document Structure Config (Elixir)"
config = %Xberg.ExtractionConfig{
  include_document_structure: true
}

{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, config)

result = List.first(output.results)
if result.document do
  Enum.each(result.document.nodes, fn node ->
    IO.puts("[#{node.content.node_type}]")
  end)
end
```
