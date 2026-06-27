```elixir title="Elixir"
defmodule InstallVerify do
  def verify_install do
    # Verify Xberg module is available
    {:ok, extractors} = Xberg.list_document_extractors()
    IO.puts("Available extractors: #{inspect(extractors)}")

    # Verify a simple extraction works
    case Xberg.extract_sync("test.txt", nil, nil) do
      {:ok, _result} ->
        IO.puts("Xberg is properly installed and working!")

      {:error, reason} ->
        IO.puts("Extraction failed: #{reason}")
    end
  end
end
```
