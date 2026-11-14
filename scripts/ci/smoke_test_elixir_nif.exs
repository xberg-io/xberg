defmodule Runner do
  @moduledoc false

  def fail(message) do
    IO.puts(:stderr, "SMOKE TEST FAILED: #{message}")
    System.halt(1)
  end

  def run([fixture_path, expected_substring]) do
    Mix.start()
    ebin = Path.expand("xberg/ebin")
    File.mkdir_p!(Path.expand("../priv/native", ebin))
    File.mkdir_p!(ebin)
    true = Code.prepend_path(ebin)

    Mix.Project.in_project(:xberg, File.cwd!(), fn _project ->
      Code.compile_file("/package/lib/xberg/native.ex")
    end)

    {:module, Xberg.Native} = Code.ensure_loaded(Xberg.Native)

    input_json = ~s({"kind":"uri","uri":#{Kernel.inspect(fixture_path)}})

    case apply(Xberg.Native, :extract_async, [input_json, nil]) do
      {:ok, %{results: [first | _]}} ->
        content = Map.get(first, :content)

        if is_binary(content) and String.contains?(content, expected_substring) do
          IO.puts("SMOKE TEST PASSED: found #{inspect(expected_substring)} in extracted content")
        else
          fail(
            "expected substring #{inspect(expected_substring)} not found in extracted content " <>
              "for fixture #{fixture_path}; got: #{inspect(content)}"
          )
        end

      {:ok, %{results: []}} ->
        fail("extract_async returned zero results for fixture #{fixture_path}")

      {:error, reason} ->
        fail("extract_async returned an error for fixture #{fixture_path}: #{inspect(reason)}")
    end
  end

  def run(_args) do
    fail("usage: elixir smoke_test_elixir_nif.exs <fixture-path> <expected-substring>")
  end
end

Runner.run(System.argv())
