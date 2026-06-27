```elixir title="Elixir"
config =
  %{
    "language_detection" => %{
      "enabled" => true,
      "min_confidence" => 0.8,
      "detect_multiple" => true
    }
  }
  |> Jason.encode!()

case Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "multilingual_document.pdf"}, config) do
  {:ok, output} ->
    result = List.first(output.results)
    languages = result.detected_languages || []

    case languages do
      [_ | _] ->
        IO.inspect(languages, label: "Detected languages")

      [] ->
        IO.puts("No language detection results")
    end

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end
```
