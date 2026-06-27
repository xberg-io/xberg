```elixir title="Elixir"
# Access different parts of result
# Content, metadata, tables, images

{:ok, result} = Xberg.extract("document.pdf")

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
