```elixir title="Elixir"
alias Xberg.ExtractionConfig

config = %ExtractionConfig{
  ocr: %{"enabled" => true, "backend" => "paddle-ocr", "language" => "en"}
}

{:ok, result} = Xberg.extract("scanned.pdf", nil, config)

for element <- result.ocr_elements || [] do
  IO.puts("Text: #{element.text}")
  IO.puts("Confidence: #{Float.round(element.confidence.recognition, 2)}")
  IO.puts("Geometry: #{inspect(element.geometry)}")

  if element.rotation do
    IO.puts("Rotation: #{element.rotation.angle}°")
  end

  IO.puts("")
end
```
