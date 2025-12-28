# Extract with chunking and track page boundaries
config = %Kreuzberg.ExtractionConfig{
  chunking: %{"enabled" => true, "max_chars" => 500},
  track_page_boundaries: true
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

# Map each chunk to its source page
Enum.with_index(result.chunks || [], 1) |> Enum.each(fn {chunk, idx} ->
  page = chunk["page"] || "unknown"
  IO.puts("Chunk #{idx} from page #{page}")
end)
