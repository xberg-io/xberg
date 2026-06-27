```elixir title="Elixir"
defmodule VectorRecord do
  defstruct [:id, :content, :embedding, :metadata]
end

defmodule VectorIntegration do
  def extract_and_vectorize(document_path, document_id) do
    config =
      %{
        "chunking" => %{
          "max_characters" => 512,
          "overlap" => 50,
          "embedding" => %{
            "model" => %{"preset" => %{"name" => "balanced"}},
            "normalize" => true,
            "batch_size" => 32
          }
        }
      }
      |> Jason.encode!()

    case Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: document_path}, config) do
      {:ok, output} ->
        result = List.first(output.results)
        chunks = result.chunks || []

        case chunks do
          [_ | _] ->
            records =
              chunks
              |> Enum.with_index()
              |> Enum.flat_map(fn {chunk, index} ->
                case chunk do
                  %{"embedding" => embedding, "content" => content}
                  when is_list(embedding) ->
                    metadata = %{
                      "document_id" => document_id,
                      "chunk_index" => Integer.to_string(index),
                      "content_length" => Integer.to_string(String.length(content))
                    }

                    [
                      %VectorRecord{
                        id: "#{document_id}_chunk_#{index}",
                        content: content,
                        embedding: embedding,
                        metadata: metadata
                      }
                    ]

                  _ ->
                    []
                end
              end)

            {:ok, records}

          [] ->
            {:error, "No chunks in extraction result"}
        end

      {:error, reason} ->
        {:error, reason}
    end
  end
end
```
