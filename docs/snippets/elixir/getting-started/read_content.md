```elixir title="Elixir"
defmodule ReadContent do
  def process_extracted_content do
    # Extract content and iterate over lines
    case Kreuzberg.extract_file_sync("document.pdf", nil, nil) do
      {:ok, content} ->
        IO.puts("Processing extracted content:")

        # Split content into lines and iterate
        content
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
