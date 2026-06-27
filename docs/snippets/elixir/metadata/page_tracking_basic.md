```elixir title="Elixir"
config =
  %{"pages" => %{"extract_pages" => true}}
  |> Jason.encode!()

case Xberg.extract_sync("document.pdf", nil, config) do
  {:ok, result} ->
    decoded = Jason.decode!(result)

    case decoded do
      %{"pages" => pages} when is_list(pages) ->
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

      _ ->
        nil
    end

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end
```
