defmodule Kreuzberg.EmbeddingConfig do
  @moduledoc """
  Configuration for standalone text embedding generation.
  """

  defstruct model: "balanced",
            normalize: true,
            batch_size: nil

  @type t :: %__MODULE__{
          model: String.t() | map(),
          normalize: boolean(),
          batch_size: pos_integer() | nil
        }

  @doc """
  Creates a new EmbeddingConfig with default values.
  """
  def new(opts \\ []) do
    struct(__MODULE__, opts)
  end

  @doc """
  Converts the configuration to a map for NIF serialization.
  """
  def to_map(%__MODULE__{} = config) do
    %{
      "model" => normalize_model(config.model),
      "normalize" => config.normalize,
      "batch_size" => config.batch_size
    }
    |> Enum.reject(fn {_, v} -> is_nil(v) end)
    |> Map.new()
  end

  defp normalize_model(name) when is_binary(name) do
    %{"type" => "preset", "name" => name}
  end

  defp normalize_model(map) when is_map(map), do: map
end
