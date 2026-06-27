```elixir title="Elixir"
defmodule HelloWorld do
  def main do
    case Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, nil) do
      {:ok, output} ->
        result = List.first(output.results)
        IO.puts("Extraction succeeded!")
        IO.puts(result.content)

      {:error, reason} ->
        IO.puts("Error: #{reason}")
    end
  end
end
```
