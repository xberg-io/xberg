```elixir title="Elixir"
defmodule Example do
  def extract_file do
    config = nil

    case Kreuzberg.extract_file_sync("document.pdf", nil, config) do
      {:ok, result} ->
        IO.puts("Content: #{result}")
        :ok

      {:error, reason} ->
        IO.puts("Error: #{reason}")
        :error
    end
  end
end
```
