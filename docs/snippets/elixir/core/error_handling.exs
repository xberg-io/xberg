```elixir title="Elixir"
# Example: Handling extraction errors
case Kreuzberg.extract_file("document.pdf") do
  {:ok, result} ->
    IO.puts("Successfully extracted content")
    IO.puts("Content length: #{byte_size(result.content)} characters")

  {:error, reason} ->
    IO.puts("Extraction failed: #{reason}")
end

# Example: Handling with custom error message
result = Kreuzberg.extract_file("nonexistent.pdf")

case result do
  {:ok, data} ->
    IO.puts("File processed successfully")
  {:error, error} ->
    IO.puts("Error details: #{inspect(error)}")
end

# Example: Extract with pattern matching
case Kreuzberg.extract(<<>>, "application/pdf") do
  {:ok, result} ->
    IO.puts("Content: #{result.content}")
  {:error, msg} when is_binary(msg) ->
    IO.puts("Validation error: #{msg}")
  {:error, reason} ->
    IO.puts("Unknown error: #{inspect(reason)}")
end
```
