```elixir title="Elixir"
defmodule MyApp.CloudOcrBackend do
  @behaviour Kreuzberg.Plugin

  defstruct api_key: nil, supported_langs: []

  def new(api_key, supported_langs) do
    %__MODULE__{api_key: api_key, supported_langs: supported_langs}
  end

  @impl Kreuzberg.Plugin
  def name(_backend), do: "cloud-ocr"

  @impl Kreuzberg.Plugin
  def version(_backend), do: "1.0.0"

  @impl Kreuzberg.Plugin
  def initialize(_backend), do: :ok

  @impl Kreuzberg.Plugin
  def shutdown(_backend), do: :ok

  def process_image(backend, image_bytes, language) do
    call_cloud_api(backend, image_bytes, language)
  end

  def supports_language(backend, lang) do
    Enum.member?(backend.supported_langs, lang)
  end

  defp call_cloud_api(_backend, _image, _language) do
    {:ok, "Extracted text"}
  end
end

# Register the custom backend
backend = MyApp.CloudOcrBackend.new("api-key", ["en", "de", "fr"])
# Use with Kreuzberg extraction...
```
