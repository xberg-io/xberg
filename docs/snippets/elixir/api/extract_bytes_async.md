```elixir title="Elixir"
defmodule Example do
  def extract_from_bytes_async do
    content = File.read!("document.pdf")
    config = nil

    task = Task.async(fn ->
      Kreuzberg.extract_bytes_async(content, "application/pdf", config)
    end)

    case Task.await(task) do
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
