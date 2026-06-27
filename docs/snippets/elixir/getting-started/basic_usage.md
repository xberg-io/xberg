```elixir title="Elixir"
defmodule BasicUsage do
  def extract_with_default_config do
    # Use default configuration (nil)
    config = nil

    case Xberg.extract_sync("document.pdf", nil, config) do
      {:ok, content} ->
        IO.puts("Extracted content:")
        IO.puts(content)
        :ok

      {:error, reason} ->
        IO.puts("Extraction failed: #{reason}")
        :error
    end
  end
end
```
