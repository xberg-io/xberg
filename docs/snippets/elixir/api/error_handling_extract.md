```elixir title="Elixir"
defmodule Example do
  def robust_extract(path) do
    with {:file_exists, true} <- {:file_exists, File.exists?(path)},
         {:read, {:ok, content}} <- {:read, File.read(path)},
         {:mime, {:ok, mime_type}} <- {:mime, detect_mime_type(content)},
         input = %Xberg.ExtractInput{kind: :bytes, bytes: content, mime_type: mime_type},
         {:extract, {:ok, output}} <- {:extract, Xberg.extract(input, nil)} do
      result = List.first(output.results)
      {:ok, result}
    else
      {:file_exists, false} ->
        {:error, "File not found: #{path}"}

      {:read, {:error, reason}} ->
        {:error, "Failed to read file: #{inspect(reason)}"}

      {:mime, {:error, reason}} ->
        {:error, "MIME detection failed: #{reason}"}

      {:extract, {:error, reason}} ->
        {:error, "Extraction failed: #{reason}"}
    end
  end

  defp detect_mime_type(content) do
    Xberg.detect_mime_type_from_bytes(content)
  end
end
```
