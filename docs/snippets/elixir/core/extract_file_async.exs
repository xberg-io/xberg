```elixir title="Elixir"
task = Kreuzberg.extract_file_async("document.pdf")
{:ok, result} = Task.await(task)

content = result.content
table_count = length(result.tables)
metadata = result.metadata

IO.puts("Content length: #{byte_size(content)} characters")
IO.puts("Tables: #{table_count}")
IO.puts("Metadata keys: #{inspect(Map.keys(metadata))}")
```
