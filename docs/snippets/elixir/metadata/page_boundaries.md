```elixir title="Elixir"
config = Jason.encode!(%{})

case Xberg.extract_sync("document.pdf", nil, config) do
  {:ok, result} ->
    decoded = Jason.decode!(result)

    case decoded do
      %{"metadata" => %{"pages" => %{"boundaries" => boundaries}}, "content" => content}
      when is_list(boundaries) ->
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

      _ ->
        nil
    end

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end
```
