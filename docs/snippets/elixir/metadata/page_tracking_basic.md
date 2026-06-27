```elixir title="Elixir"
config =
  %{"pages" => %{"extract_pages" => true}}
  |> Jason.encode!()

case Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, config) do
  {:ok, output} ->
    result = List.first(output.results)
    pages = result.pages || []

    case pages do
      [_ | _] ->
        Enum.each(pages, fn page ->
          page_number = page["page_number"]
          content = page["content"]
          tables = page["tables"] || []
          images = page["images"] || []

          IO.puts("Page #{page_number}:")
          IO.puts("  Content: #{String.length(content)} chars")
          IO.puts("  Tables: #{length(tables)}")
          IO.puts("  Images: #{length(images)}")
        end)

      [] ->
        nil
    end

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end
```
