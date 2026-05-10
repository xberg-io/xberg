```elixir title="Elixir"
defmodule Example do
  def chunk_text_via_http do
    text = "Your long document text here..."

    config = %{
      "chunking" => %{
        "max_characters" => 800,
        "overlap" => 100,
        "chunker_type" => "Markdown"
      }
    }

    with {:ok, response} <- Req.post(
           "http://localhost:8000/chunk",
           json: %{
             "text" => text,
             "config" => config
           }
         ),
         {:ok, body} <- Jason.decode(response.body) do
      chunks = body["chunks"]
      IO.puts("Created #{length(chunks)} chunks")
      {:ok, chunks}
    else
      error -> {:error, inspect(error)}
    end
  end
end
```
