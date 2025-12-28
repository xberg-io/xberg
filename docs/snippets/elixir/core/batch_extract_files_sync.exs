```elixir title="Elixir"
file_paths = ["document1.pdf", "document2.pdf", "document3.pdf"]

{:ok, results} = Kreuzberg.batch_extract_files(file_paths)

Enum.each(results, fn result ->
  IO.puts("File: #{result.mime_type}")
  IO.puts("Content length: #{byte_size(result.content)} characters")
  IO.puts("Tables: #{length(result.tables)}")
  IO.puts("---")
end)

IO.puts("Total files processed: #{length(results)}")
```
