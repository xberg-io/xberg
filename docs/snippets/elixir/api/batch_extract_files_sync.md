```elixir title="Elixir"
defmodule Example do
  def batch_extract_files do
    files = ["doc1.pdf", "doc2.docx", "report.pdf"]
    config = nil

    results =
      files
      |> Task.async_stream(
        fn file ->
          Kreuzberg.extract_file_sync(file, nil, config)
        end,
        max_concurrency: 4
      )
      |> Enum.map(fn {:ok, result} -> result end)

    Enum.each(results, fn
      {:ok, content} -> IO.puts("File extracted: #{String.length(content)} chars")
      {:error, reason} -> IO.puts("Error: #{reason}")
    end)
  end
end
```
