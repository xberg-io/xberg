defmodule Jason.Mixfile do
  use Mix.Project

  @source_url "https://github.com/michalmuskala/jason"
  @version "1.4.5"

  def project() do
    [
      app: :jason,
      version: @version,
      elixir: "~> 1.4",
      start_permanent: Mix.env() == :prod,
      consolidate_protocols: Mix.env() != :test,
      deps: deps(),
      preferred_cli_env: [docs: :docs],
      dialyzer: dialyzer(),
      description: description(),
      package: package(),
      docs: docs()
    ]
  end

  def application() do
    [
      extra_applications: []
    ]
  end

  defp deps() do
    [
      {:decimal, "~> 1.0 or ~> 2.0 or ~> 3.0", optional: true},
      {:dialyxir, "~> 1.0", only: [:dev, :test], runtime: false},
      {:ex_doc, ">= 0.0.0", only: :dev, runtime: false},
    ] ++ maybe_stream_data()
  end

  defp maybe_stream_data() do
    if Version.match?(System.version(), "~> 1.12") do
      [{:stream_data, "~> 1.0", only: :test}]
    else
      []
    end
  end

  defp dialyzer() do
    [
      plt_add_apps: [:decimal]
    ]
  end

  defp description() do
    """
    A blazing fast JSON parser and generator in pure Elixir.
    """
  end

  defp package() do
    [
      maintainers: ["Michał Muskała"],
      licenses: ["Apache-2.0"],
      links: %{"GitHub" => @source_url}
    ]
  end

  defp docs() do
    [
      main: "readme",
      name: "Jason",
      source_ref: "v#{@version}",
      canonical: "http://hexdocs.pm/jason",
      source_url: @source_url,
      extras: ["README.md", "CHANGELOG.md", "LICENSE"]
    ]
  end
end
