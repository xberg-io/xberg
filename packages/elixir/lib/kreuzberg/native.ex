defmodule Kreuzberg.Native do
  @moduledoc false

  use Rustler,
    otp_app: :kreuzberg,
    crate: "kreuzberg_rustler",
    mode: if(Mix.env() == :prod, do: :release, else: :debug)

  # Basic extraction
  def extract(_input, _input_type), do: :erlang.nif_error(:nif_not_loaded)
end
