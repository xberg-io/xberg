defmodule E2eElixir.MixProject do
  use Mix.Project

  def project do
    [
      app: :e2e_elixir,
      version: "0.1.0",
      elixir: "~> 1.14",
      deps: deps()
    ]
  end

  defp deps do
    [
      {:xberg, path: "../../packages/elixir"},
      {:rustler_precompiled, "~> 0.9"},
      {:rustler, "~> 0.37", runtime: false},
      {:finch, "~> 0.18"},
      {:req, "~> 0.5"},
      {:jason, "~> 1.4"}
    ]
  end
end
