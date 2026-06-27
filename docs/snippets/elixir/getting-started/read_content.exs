```elixir title="Elixir"
# Access different parts of result
# Content, metadata, tables, images

{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, nil)

result = List.first(output.results)
# Access main content
content = result.content
IO.puts("Content length: #{String.length(content)} characters")

# Access tables
tables = result.tables
IO.puts("Tables found: #{length(tables)}")

# Access images
images = result.images
IO.puts("Images found: #{length(images)}")

# Access metadata
format = result.metadata.format
IO.puts("Format: #{inspect(format)}")
```
