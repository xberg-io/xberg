```elixir title="Elixir"
# Extract from different file types (PDF, DOCX, etc.)

case Kreuzberg.extract_file("document.pdf") do
  {:ok, result} ->
    IO.puts("Content: #{result.content}")
    IO.puts("MIME Type: #{result.metadata.format_type}")
    IO.puts("Tables: #{length(result.tables)}")

  {:error, reason} ->
    IO.puts("Extraction failed: #{inspect(reason)}")
end
```
