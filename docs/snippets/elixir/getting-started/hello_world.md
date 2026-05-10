```elixir title="Elixir"
defmodule HelloWorld do
  def main do
    case Kreuzberg.extract_file_sync("document.pdf", nil, nil) do
      {:ok, result} ->
        IO.puts("Extraction succeeded!")
        IO.puts(result)

      {:error, reason} ->
        IO.puts("Error: #{reason}")
    end
  end
end
```
