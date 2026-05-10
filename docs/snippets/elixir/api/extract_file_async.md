```elixir title="Elixir"
defmodule Example do
  def extract_file_async do
    config = nil

    task = Task.async(fn ->
      Kreuzberg.extract_file_async("document.pdf", nil, config)
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
