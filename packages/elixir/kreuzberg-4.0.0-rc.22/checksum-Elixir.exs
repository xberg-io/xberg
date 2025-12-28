defmodule RustlerPrecompiled.ChecksumGenerator do
  @moduledoc """
  Generates checksums for precompiled NIFs downloaded from GitHub releases.

  This script is used during the release process to generate SHA256 checksums
  for all platform-specific binaries. The checksums are used by rustler_precompiled
  to verify the integrity of downloaded binaries.

  Usage:
    mix run checksum-Elixir.exs
  """

  def generate do
    version = Mix.Project.config()[:version]
    base_url = "https://github.com/kreuzberg-dev/kreuzberg/releases/download/v#{version}"

    targets = [
      "aarch64-apple-darwin",
      "x86_64-apple-darwin",
      "x86_64-unknown-linux-gnu",
      "x86_64-unknown-linux-musl",
      "aarch64-unknown-linux-gnu",
      "aarch64-unknown-linux-musl",
      "x86_64-pc-windows-msvc",
      "x86_64-pc-windows-gnu"
    ]

    nif_versions = ["2.16", "2.17"]

    IO.puts("Generating checksums for Kreuzberg v#{version}\n")
    IO.puts("Base URL: #{base_url}\n")

    checksums =
      for target <- targets,
          nif_version <- nif_versions do
        ext = if String.contains?(target, "windows"), do: "dll", else: "so"
        filename = "libkreuzberg_rustler-v#{version}-nif-#{nif_version}-#{target}.#{ext}.tar.gz"
        url = "#{base_url}/#{filename}"

        case download_and_checksum(url, filename) do
          {:ok, checksum} ->
            IO.puts("✓ #{filename}: #{checksum}")
            {filename, checksum}

          {:error, reason} ->
            IO.puts("✗ #{filename}: #{reason}")
            nil
        end
      end
      |> Enum.reject(&is_nil/1)
      |> Map.new()

    if Enum.empty?(checksums) do
      IO.puts("\n⚠ Warning: No checksums generated!")
      IO.puts("Make sure binaries are published to GitHub releases first.")
    else
      write_checksum_file(checksums)
      IO.puts("\n✓ Checksums written to checksum-Elixir.exs")
      IO.puts("Total files: #{map_size(checksums)}")
    end
  end

  defp download_and_checksum(url, filename) do
    cache_dir = Path.join([System.tmp_dir!(), "kreuzberg-checksums"])
    File.mkdir_p!(cache_dir)
    local_path = Path.join(cache_dir, filename)

    # Try to download the file
    case System.cmd("curl", ["-fsSL", "-o", local_path, url], stderr_to_stdout: true) do
      {_, 0} ->
        # Calculate SHA256
        case System.cmd("shasum", ["-a", "256", local_path]) do
          {output, 0} ->
            [checksum | _] = String.split(output)
            {:ok, String.trim(checksum)}

          {error, _} ->
            {:error, "Failed to calculate checksum: #{error}"}
        end

      {error, _} ->
        {:error, "Download failed: #{String.slice(error, 0..100)}"}
    end
  end

  defp write_checksum_file(checksums) do
    content = """
    %{
    #{Enum.map_join(checksums, ",\n", fn {filename, hash} ->
      "  \"#{filename}\" => \"sha256:#{hash}\""
    end)}
    }
    """

    File.write!("checksum-Elixir.exs", content)
  end
end

# Run the generator
RustlerPrecompiled.ChecksumGenerator.generate()
