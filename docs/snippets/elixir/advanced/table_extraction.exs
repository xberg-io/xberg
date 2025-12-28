```elixir title="Elixir"
{:ok, result} = Kreuzberg.extract_file("document.pdf")

tables = result.tables
IO.puts("Total tables found: #{length(tables)}")

Enum.with_index(tables, 1) |> Enum.each(fn {table, index} ->
  IO.puts("\n--- Table #{index} ---")

  # Access table cells
  cells = table["cells"] || []
  IO.puts("Rows: #{length(cells)}")

  # Access table markdown representation
  markdown = table["markdown"]
  IO.puts("Markdown representation:")
  IO.puts(markdown)
end)
```
