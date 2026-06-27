```elixir title="Elixir"
config = Jason.encode!(%{})

case Xberg.extract_sync("document.pdf", nil, config) do
  {:ok, result} ->
    decoded = Jason.decode!(result)

    case decoded do
      %{"tables" => tables} when is_list(tables) ->
        Enum.each(tables, fn table ->
          cells = table["cells"] || []
          markdown = table["markdown"] || ""

          IO.puts("Table with #{length(cells)} rows")
          IO.puts("#{markdown}")

          Enum.each(cells, fn row ->
            IO.inspect(row)
          end)
        end)

      _ ->
        nil
    end

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end
```
