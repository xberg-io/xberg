# Extract with chunking and track page boundaries
config = %Xberg.ExtractionConfig{
  chunking: %{"enabled" => true, "max_characters" => 500},
  track_page_boundaries: true
}

{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, config)

result = List.first(output.results)
# Map each chunk to its source page
Enum.with_index(result.chunks || [], 1) |> Enum.each(fn {chunk, idx} ->
  page = chunk["page"] || "unknown"
  IO.puts("Chunk #{idx} from page #{page}")
end)
