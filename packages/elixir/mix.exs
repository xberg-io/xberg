defmodule Kreuzberg.MixProject do
  use Mix.Project

  def project do
    [
      app: :kreuzberg,
      version: "5.0.0-rc.1",
      elixir: "~> 1.14",
      elixirc_paths: ["lib", Path.expand("../../packages/elixir/native/kreuzberg_nif/src", __DIR__)],
      rustler_crates: [kreuzberg_nif: [mode: :release]],
      description: "High-performance document intelligence library",
      package: package(),
      deps: deps()
    ]
  end

  defp package do
    [
      licenses: ["Elastic-2.0"],
      links: %{"GitHub" => "https://github.com/kreuzberg-dev/kreuzberg"},
      files:
        ~w(lib .formatter.exs mix.exs README* checksum-*.exs native/kreuzberg_nif/Cargo.toml native/kreuzberg_nif/Cargo.lock native/kreuzberg_nif/src)
    ]
  end

  defp deps do
    [
      {:jason, "~> 1.4"},
      {:rustler, "~> 0.37.0", runtime: false},
      {:rustler_precompiled, "~> 0.9"},
      {:credo, "~> 1.7", only: [:dev, :test], runtime: false},
      {:ex_doc, "~> 0.40", only: :dev, runtime: false}
    ]
  end
end
