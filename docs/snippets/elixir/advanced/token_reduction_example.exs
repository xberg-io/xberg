```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Reduce token count for LLM
config = %ExtractionConfig{
  token_reduction: %{
    "enabled" => true,
    "target_tokens" => 2000
  }
}

case Kreuzberg.extract_file("document.pdf", nil, config) do
  {:ok, result} ->
    IO.puts("=== Token Reduction ===\n")

    # Display content and token information
    content_size = byte_size(result.content)
    estimated_tokens = div(content_size, 4)  # Rough estimate: 1 token ≈ 4 bytes

    IO.puts("Content size: #{content_size} bytes")
    IO.puts("Estimated tokens: ~#{estimated_tokens}")
    IO.puts("Target tokens: 2000")

    # Show reduction status
    if estimated_tokens > 2000 do
      reduction_percentage = trunc((1 - 2000 / estimated_tokens) * 100)
      IO.puts("\nToken reduction applied: ~#{reduction_percentage}% reduction")
    else
      IO.puts("\nNo reduction needed - content already below target")
    end

    # Display reduced content preview
    IO.puts("\n--- Reduced Content ---")
    content_preview = String.slice(result.content, 0..300)
    IO.puts(content_preview)
    IO.puts("...")

  {:error, reason} ->
    IO.puts("Extraction failed!")
    IO.puts("Error: #{inspect(reason)}")
end
```
