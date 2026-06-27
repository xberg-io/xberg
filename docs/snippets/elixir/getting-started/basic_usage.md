```elixir title="Elixir"
defmodule BasicUsage do
  def extract_with_default_config do
    # Use default configuration (nil)
    config = nil

    case Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, config) do
      {:ok, output} ->
        result = List.first(output.results)
        IO.puts("Extracted content:")
        IO.puts(result.content)
        :ok

      {:error, reason} ->
        IO.puts("Extraction failed: #{reason}")
        :error
    end
  end
end
```
