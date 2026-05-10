```elixir title="Elixir"
defmodule Example do
  def extract_via_http do
    file_path = "document.pdf"

    with {:ok, file} <- File.read(file_path),
         {:ok, response} <- Req.post(
           "http://localhost:8000/extract",
           form: [file: {:file, file_path}]
         ),
         {:ok, body} <- Jason.decode(response.body) do
      IO.puts("Extracted content: #{body["content"]}")
      {:ok, body}
    else
      error -> {:error, inspect(error)}
    end
  end
end
```
