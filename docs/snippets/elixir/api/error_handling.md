```elixir title="Elixir"
defmodule Example do
  def handle_extraction_errors do
    # Extract with invalid MIME type
    case Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.txt"}, nil) do
      {:ok, output} ->
        result = List.first(output.results)
        IO.puts("Success: #{String.length(result.content || "")} chars")

      {:error, reason} when is_binary(reason) ->
        # Error is a string description
        case reason do
          msg when String.contains?(msg, "unsupported") ->
            IO.puts("Unsupported format: #{msg}")

          msg when String.contains?(msg, "not found") ->
            IO.puts("File not found: #{msg}")

          msg ->
            IO.puts("Extraction failed: #{msg}")
        end
    end
  end
end
```
