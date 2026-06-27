```elixir title="Elixir"
# Basic document extraction workflow
# Load file -> extract -> access results

{:ok, result} = Xberg.extract("document.pdf")

IO.puts("Extracted Content:")
IO.puts(result.content)

IO.puts("\nMetadata:")
IO.puts("Format: #{inspect(result.metadata.format)}")
IO.puts("Tables found: #{length(result.tables)}")
```
