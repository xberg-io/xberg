defmodule Kreuzberg.MixProject do
  use Mix.Project

  @version "4.2.1"
  @source_url "https://github.com/kreuzberg-dev/kreuzberg"

  def project do
    [
      app: :kreuzberg,
      version: @version,
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      elixirc_paths: elixirc_paths(Mix.env()),
      deps: deps(),
      description: "High-performance document intelligence library with OCR support",
      package: package(),
      docs: docs(),
      source_url: @source_url,
      rustler_crates: [kreuzberg: [mode: :release]]
    ]
  end

  def application do
    [
      extra_applications: [:logger],
      mod: {Kreuzberg.Application, []}
    ]
  end

  defp deps do
    [
      {:jason, "~> 1.4", runtime: false},
      {:rustler, "~> 0.37.0", optional: true, runtime: false},
      {:rustler_precompiled, "~> 0.8"},
      {:credo, "~> 1.7", only: [:dev, :test], runtime: false},
      {:ex_doc, "~> 0.34", only: :dev, runtime: false}
    ]
  end

  defp package do
    [
      licenses: ["MIT"],
      links: %{GitHub: @source_url},
      files: ~w(
        lib
        native/kreuzberg_rustler/src
        native/kreuzberg_rustler/Cargo.toml
        native/kreuzberg_rustler/Cargo.lock
        mix.exs
        README.md
        .formatter.exs
        checksum-Elixir.Kreuzberg.Native.exs
      )
    ]
  end

  defp docs do
    [
      main: "Kreuzberg",
      source_url: @source_url,
      extras: ["README.md"]
    ]
  end

  defp elixirc_paths(:test), do: ["lib", "test/support"]
  defp elixirc_paths(_), do: ["lib"]
end
