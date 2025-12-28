```elixir title="Elixir"
# Access different parts of result
# Content, metadata, tables, images

{:ok, result} = Kreuzberg.extract_file("document.pdf")

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
format_type = result.metadata.format_type
IO.puts("Format: #{inspect(format_type)}")
```
