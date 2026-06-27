```elixir title="Elixir"
defmodule ReadContent do
  def process_extracted_content do
    # Extract content and iterate over lines
    case Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, nil) do
      {:ok, output} ->
        result = List.first(output.results)
        IO.puts("Processing extracted content:")

        # Split content into lines and iterate
        result.content
        |> String.split("\n", trim: true)
        |> Enum.each(fn line ->
          IO.puts("  #{line}")
        end)

        :ok

      {:error, reason} ->
        IO.puts("Error: #{reason}")
        :error
    end
  end
end
```
