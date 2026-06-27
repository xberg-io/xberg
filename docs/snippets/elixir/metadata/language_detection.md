```elixir title="Elixir"
config =
  %{
    "language_detection" => %{
      "enabled" => true,
      "min_confidence" => 0.9,
      "detect_multiple" => true
    }
  }
  |> Jason.encode!()

case Xberg.extract_sync("document.pdf", nil, config) do
  {:ok, result} ->
    decoded = Jason.decode!(result)

    case decoded do
      %{"detected_languages" => languages} when is_list(languages) ->
        IO.inspect(languages, label: "Detected languages")

      _ ->
        IO.puts("No language detection results")
    end

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end
```
