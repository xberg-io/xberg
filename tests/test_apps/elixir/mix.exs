defmodule KreuzbergTestApp.MixProject do
  use Mix.Project

  def project do
    [
      app: :kreuzberg_test_app,
      version: "4.2.1",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_paths: ["test"],
      elixirc_paths: elixirc_paths(Mix.env())
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:kreuzberg, path: "../../../packages/elixir", override: true},
      {:rustler, ">= 0.0.0", optional: true}
    ]
  end

  defp elixirc_paths(:test), do: ["lib", "test/support"]
  defp elixirc_paths(_), do: ["lib"]
end
