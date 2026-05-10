```elixir title="Elixir"
defmodule Example do
  def batch_extract_bytes do
    # Note: Batch extraction in Elixir is done via Task.async_stream over sync calls
    files = ["doc1.pdf", "doc2.docx", "report.pdf"]
    config = nil

    results =
      files
      |> Task.async_stream(
        fn file ->
          content = File.read!(file)
          Kreuzberg.extract_bytes_sync(content, "application/pdf", config)
        end,
        max_concurrency: 4
      )
      |> Enum.map(fn {:ok, result} -> result end)

    Enum.each(results, fn
      {:ok, content} -> IO.puts("Extracted: #{String.length(content)} chars")
      {:error, reason} -> IO.puts("Error: #{reason}")
    end)
  end
end
```
