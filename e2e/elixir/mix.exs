defmodule E2eElixir.MixProject do
  use Mix.Project

  def project do
    [
      app: :e2e_elixir,
      version: "0.1.0",
      elixir: "~> 1.14",
      # Absolute test path so the suite still compiles after test_helper.exs
      # chdir's into test_documents for relative file-URI resolution.
      test_paths: [Path.expand("test", __DIR__)],
      deps: deps(),
      rustler_precompiled: [
        force_build: System.get_env("XBERG_BUILD") in ["1", "true"] or Mix.env() in [:dev, :test]
      ]
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
