```elixir title="Elixir"
{:ok, result} = Kreuzberg.extract_file("document.pdf")

content = result.content
table_count = length(result.tables)
metadata = result.metadata

IO.puts("Content length: #{byte_size(content)} characters")
IO.puts("Tables: #{table_count}")
IO.puts("Metadata keys: #{inspect(Map.keys(metadata))}")
```
