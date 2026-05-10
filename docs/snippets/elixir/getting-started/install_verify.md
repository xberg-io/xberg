```elixir title="Elixir"
defmodule InstallVerify do
  def verify_install do
    # Verify Kreuzberg module is available
    {:ok, extractors} = Kreuzberg.list_document_extractors()
    IO.puts("Available extractors: #{inspect(extractors)}")

    # Verify a simple extraction works
    case Kreuzberg.extract_file_sync("test.txt", nil, nil) do
      {:ok, _result} ->
        IO.puts("Kreuzberg is properly installed and working!")

      {:error, reason} ->
        IO.puts("Extraction failed: #{reason}")
    end
  end
end
```
