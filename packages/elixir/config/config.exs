import Config

# Send all log output to stderr so stdout remains clean for JSON output
# (benchmarks and other tools that parse stdout)
config :logger, :default_handler, config: %{type: :standard_error}

config :rustler_precompiled, :force_build,
  kreuzberg: System.get_env("KREUZBERG_BUILD") in ["1", "true"] || Mix.env() in [:test, :dev]
