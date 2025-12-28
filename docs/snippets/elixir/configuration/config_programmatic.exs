```elixir title="Elixir"
alias Kreuzberg.ExtractionConfig

# Build configuration dynamically based on runtime conditions
# Useful for environment-specific settings and feature flags

defmodule ConfigBuilder do
  def build_config(file_type, enable_ocr?) do
    base_config = %ExtractionConfig{
      chunking: %{"max_chars" => 1000, "max_overlap" => 100},
      use_cache: true
    }

    case {file_type, enable_ocr?} do
      {:pdf, true} ->
        %{base_config | ocr: %{"enabled" => true, "backend" => "tesseract"}, force_ocr: true}

      {:pdf, false} ->
        %{base_config | ocr: %{"enabled" => false}}

      {:image, true} ->
        %{
          base_config
          | ocr: %{"enabled" => true, "backend" => "tesseract", "preprocessing" => true},
            force_ocr: true
        }

      {:image, false} ->
        %{base_config | ocr: %{"enabled" => false}}

      {_, _} ->
        base_config
    end
  end
end

# Build configuration based on file type and requirements
config = ConfigBuilder.build_config(:pdf, true)

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

IO.puts("Dynamic configuration applied")
IO.puts("Content: #{String.slice(result.content, 0..100)}")
```
