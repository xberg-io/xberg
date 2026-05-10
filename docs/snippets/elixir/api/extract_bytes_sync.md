```elixir title="Elixir"
defmodule Example do
  def extract_from_bytes do
    content = File.read!("document.pdf")
    config = nil

    case Kreuzberg.extract_bytes_sync(content, "application/pdf", config) do
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
