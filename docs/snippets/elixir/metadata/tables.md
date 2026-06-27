```elixir title="Elixir"
config = Jason.encode!(%{})

case Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, config) do
  {:ok, output} ->
    result = List.first(output.results)
    tables = result.tables || []

    case tables do
      [_ | _] ->
        Enum.each(tables, fn table ->
          cells = table["cells"] || []
          markdown = table["markdown"] || ""

          IO.puts("Table with #{length(cells)} rows")
          IO.puts("#{markdown}")

          Enum.each(cells, fn row ->
            IO.inspect(row)
          end)
        end)

      [] ->
        nil
    end

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end
```
