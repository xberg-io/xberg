```elixir title="Elixir"
config = Jason.encode!(%{})

case Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, config) do
  {:ok, output} ->
    result = List.first(output.results)
    boundaries = get_in(result.metadata || %{}, ["pages", "boundaries"]) || []
    content = result.content || ""

    case boundaries do
      [_ | _] ->
        boundaries
        |> Enum.take(3)
        |> Enum.each(fn boundary ->
          byte_start = boundary["byte_start"]
          byte_end = boundary["byte_end"]
          page_number = boundary["page_number"]

          # Extract substring for this boundary
          page_text = String.slice(content, byte_start, byte_end - byte_start)
          preview_end = min(100, String.length(page_text))
          preview = String.slice(page_text, 0, preview_end)

          IO.puts("Page #{page_number}:")
          IO.puts("  Byte range: #{byte_start}-#{byte_end}")
          IO.puts("  Preview: #{preview}...")
        end)

      [] ->
        nil
    end

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end
```
