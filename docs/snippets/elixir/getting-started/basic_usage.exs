```elixir title="Elixir"
# Basic document extraction workflow
# Load file -> extract -> access results

{:ok, result} = Kreuzberg.extract_file("document.pdf")

IO.puts("Extracted Content:")
IO.puts(result.content)

IO.puts("\nMetadata:")
IO.puts("Format: #{inspect(result.metadata.format_type)}")
IO.puts("Tables found: #{length(result.tables)}")
```
