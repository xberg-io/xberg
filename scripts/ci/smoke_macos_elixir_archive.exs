# Load the verified local archive without invoking RustlerPrecompiled's
# downloader or building a second NIF.
defmodule MacosElixirArchiveSmoke do
  @moduledoc false

  @use_block ~r/^  use RustlerPrecompiled,.*?force_build:[^\n]*$/ms
  @codec ~r/^lib(?:heif|x265|de265|aom)(?:\.\d+)+\.dylib$/
  @member_count 20
  @size 64

  # Synthetic 64 × 64 gradient, encoded independently through pinned x265
  # 4.3 and libaom 3.15.3. These are deliberately small non-corpus controls.
  @heic_hash "4fc4171ff78864c8d0bfdd52af01eb8f30020a636132c037a4ac9c172cfb85cc"
  @avif_hash "43b824636e65ec008ad692dff3c3f0391aca73d554ec598c274dd217cf3e1239"
  @heic """
  AAAAHGZ0eXBoZWljAAAAAG1pZjFoZWljbWlhZgAAAT9tZXRhAAAAAAAAACFoZGxyAAAAAAAAAABw
  aWN0AAAAAAAAAAAAAAAAAAAAACJpbG9jAAAAAERAAAEAAQAAAAABYwABAAAAAAAAAIEAAAAjaWlu
  ZgAAAAAAAQAAABVpbmZlAgAAAAABAABodmMxAAAAAA5waXRtAAAAAAABAAAAv2lwcnAAAAChaXBj
  bwAAAHVodmNDAQNwAAAAkAAAAAAAHvAA/P34+AAADwNgAAEAGEABDAH//wNwAAADAJAAAAMAAAMA
  HroCQGEAAQAoQgEBA3AAAAMAkAAAAwAAAwAeoCCBBZbqSSmubAgAAAMAyAAAAwAIQGIAAQAHRAHB
  crAiQAAAABRpc3BlAAAAAAAAAEAAAABAAAAAEHBpeGkAAAAAAwgICAAAABZpcG1hAAAAAAAAAAEA
  AQOBAgMAAACJbWRhdAAAAH0oAa8E+O3qDxiIn5lOBrNoIfnW8X9OxZ/FeGfTj6Kf27SKDCWFRft/
  kzYxatp+FjGdrNeoNgJsDob2S0VmQHcOcic97oS4/gynweYEpIgolcphZOSUOXj//0WUk+ho32pW
  b1tkjTfxpx4Q9AH+CJ29BF/83gwWyjO1XDe/wA==
  """
  @avif """
  AAAAHGZ0eXBhdmlmAAAAAG1pZjFhdmlmbWlhZgAAANZtZXRhAAAAAAAAACFoZGxyAAAAAAAAAABw
  aWN0AAAAAAAAAAAAAAAAAAAAACJpbG9jAAAAAERAAAEAAQAAAAAA+gABAAAAAAAAAEMAAAAjaWlu
  ZgAAAAAAAQAAABVpbmZlAgAAAAABAABhdjAxAAAAAA5waXRtAAAAAAABAAAAVmlwcnAAAAA4aXBj
  bwAAAAxhdjFDgQAMAAAAABRpc3BlAAAAAAAAAEAAAABAAAAAEHBpeGkAAAAAAwgICAAAABZpcG1h
  AAAAAAAAAAEAAQOBAgMAAABLbWRhdBIACgYYFX/9oQgyNxPHh4ZlAYYYfhUAAADCEPeSAl1aM8xc
  sQiHGbOPGGBKDEnuOfPlWlqfToze2cihQM+GNhqH9kA=
  """

  def run([mode, archive, native_source, unpacked])
      when mode in ["encode-decode", "missing-x265"] do
    verify_hosted_beam!()
    unpacked = Path.expand(unpacked)
    {nif, codecs} = extract!(Path.expand(archive), unpacked)
    if mode == "missing-x265", do: withhold_x265!(unpacked, codecs)
    load_nif!(native_source, Path.rootname(nif))

    if mode == "missing-x265", do: raise("NIF unexpectedly loaded without its x265 sibling")

    verify_codecs!(unpacked)
    verify_loaded_paths!(unpacked, codecs)
  end

  def run(_args) do
    raise("usage: elixir smoke_macos_elixir_archive.exs MODE ARCHIVE NATIVE_EX EMPTY_DIRECTORY")
  end

  defp verify_hosted_beam! do
    if System.get_env("XBERG_EXPECT_HOSTED_BEAM") == "1" do
      otp = System.otp_release()
      elixir = System.version()
      architecture = :erlang.system_info(:system_architecture) |> List.to_string()

      unless otp == "27" and String.starts_with?(elixir, "1.17.") and
               Regex.match?(~r/\A(?:aarch64|arm64)-apple-darwin(?:[0-9.]+)?\z/, architecture) do
        raise "expected Elixir 1.17 on OTP 27 arm64 macOS, got Elixir #{elixir}, OTP #{otp}, #{architecture}"
      end

      IO.puts("Verified scrubbed BEAM runtime: Elixir #{elixir}, OTP #{otp}, #{architecture}")
    end
  end

  defp digest(bytes), do: :crypto.hash(:sha256, bytes) |> Base.encode16(case: :lower)

  defp extract!(archive, directory) do
    [] = File.ls!(directory)
    bytes = File.read!(archive)
    basename = Path.basename(archive)
    hash = digest(bytes)
    checksum = "#{hash}  #{basename}\n"
    ^checksum = File.read!(archive <> ".sha256")
    :ok = :erl_tar.extract({:binary, bytes}, [:compressed, cwd: String.to_charlist(directory)])

    members = File.ls!(directory)
    count = length(members)
    @member_count = count

    for name <- ["LICENSE", "THIRD_PARTY_LICENSES.md", "sources.tsv", "notices.sha256"] do
      true = name in members
    end

    root = Path.basename(basename, ".tar.gz")
    [^root] = Enum.filter(members, &String.ends_with?(&1, ".so"))
    codecs = Enum.filter(members, &Regex.match?(@codec, &1))
    4 = length(codecs)

    IO.puts("OTP archive extraction verified #{count} members, SHA-256 #{hash}")

    {Path.join(directory, root), codecs}
  end

  defp withhold_x265!(directory, codecs) do
    [x265] = Enum.filter(codecs, &String.starts_with?(&1, "libx265."))
    File.rm!(Path.join(directory, x265))
  end

  defp load_nif!(source_path, nif) do
    source = File.read!(source_path)
    [_] = Regex.scan(@use_block, source)

    loader = """
      @on_load :load_qualified_nif
      def load_qualified_nif do
        case :erlang.load_nif(#{inspect(String.to_charlist(nif))}, 0) do
          :ok -> :ok
          {:error, reason} ->
            IO.puts(:stderr, inspect(reason, limit: :infinity))
            {:error, reason}
        end
      end
    """

    @use_block
    |> Regex.replace(source, fn _ -> loader end, global: false)
    |> Code.compile_string(source_path)

    unless Code.ensure_loaded?(Xberg.Native) and
             function_exported?(Xberg.Native, :extract_async, 2) do
      raise "could not load the verified archive NIF"
    end
  end

  defp verify_codecs!(directory) do
    pdf = Path.join(directory, "synthetic.pdf")
    round_trip = Path.join(directory, "round-trip.heic")
    heic = Path.join(directory, "independent.heic")
    avif = Path.join(directory, "independent.avif")

    File.write!(pdf, synthetic_pdf())
    write_fixture!(heic, @heic, @heic_hash)
    write_fixture!(avif, @avif, @avif_hash)

    config =
      ~s({"use_cache":false,"disable_ocr":true,"images":{"extract_images":true,"output_format":{"type":"heif","quality":80}}})

    %{
      images: [%{format: "heif", width: @size, height: @size, data: encoded}],
      processing_warnings: []
    } =
      extract_document!(pdf, config)

    encoded = IO.iodata_to_binary(encoded)
    true = byte_size(encoded) > 64
    <<_::32, "ftyp", _::binary>> = encoded
    File.write!(round_trip, encoded)
    IO.puts("Xberg HEIF encoder returned #{@size}x#{@size}, #{byte_size(encoded)} bytes")

    for image <- [round_trip, heic, avif] do
      %{
        mime_type: "image/png",
        metadata: %{format: %{image: %{format: "HEIF", width: @size, height: @size}}}
      } =
        extract_document!(image, ~s({"use_cache":false,"disable_ocr":true}))

      IO.puts("Xberg decoded #{Path.basename(image)} at #{@size}x#{@size}")
    end
  end

  defp write_fixture!(path, base64, expected_hash) do
    bytes = Base.decode64!(base64, ignore: :whitespace)
    ^expected_hash = digest(bytes)
    File.write!(path, bytes)
  end

  defp extract_document!(path, config) do
    input = ~s({"kind":"uri","uri":#{inspect(path)}})

    {:ok, %{results: [document], errors: []}} =
      apply(Xberg.Native, :extract_async, [input, config])

    document
  end

  defp verify_loaded_paths!(directory, codecs) do
    {maps, 0} = System.cmd("/usr/sbin/lsof", ["-p", System.pid(), "-Fn"])

    mapped =
      maps
      |> String.split("\n")
      |> Enum.filter(&String.starts_with?(&1, "n/"))
      |> Enum.map(&String.trim_leading(&1, "n"))
      |> Enum.filter(&Regex.match?(@codec, Path.basename(&1)))
      |> Enum.sort()

    expected = codecs |> Enum.map(&Path.join(directory, &1)) |> Enum.sort()
    ^expected = mapped
    Enum.each(mapped, &IO.puts("Loaded archive sibling #{&1}"))
  end

  defp synthetic_pdf do
    pixels =
      for y <- 0..(@size - 1), x <- 0..(@size - 1), into: <<>> do
        <<x * 4, y * 4, (x + y) * 2>>
      end

    objects = [
      "<< /Type /Catalog /Pages 2 0 R >>",
      "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
      "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 144 144] /Resources << /XObject << /Photo 4 0 R >> >> /Contents 5 0 R >>",
      stream(
        "/Type /XObject /Subtype /Image /Width #{@size} /Height #{@size} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode",
        :zlib.compress(pixels)
      ),
      stream("", "q\n144 0 0 144 0 0 cm\n/Photo Do\nQ\n")
    ]

    header = "%PDF-1.4\n"

    {body, offsets, xref_position} =
      objects
      |> Enum.with_index(1)
      |> Enum.reduce({[header], [], byte_size(header)}, fn {object, index}, accumulator ->
        {body, offsets, offset} = accumulator
        entry = ["#{index} 0 obj\n", object, "\nendobj\n"]
        {[body, entry], [offset | offsets], offset + IO.iodata_length(entry)}
      end)

    xref =
      Enum.map(
        Enum.reverse(offsets),
        &"#{String.pad_leading(Integer.to_string(&1), 10, "0")} 00000 n \n"
      )

    count = length(objects) + 1

    [
      body,
      "xref\n0 #{count}\n0000000000 65535 f \n",
      xref,
      "trailer\n<< /Size #{count} /Root 1 0 R >>\nstartxref\n#{xref_position}\n%%EOF\n"
    ]
  end

  defp stream(dictionary, bytes),
    do: ["<< #{dictionary} /Length #{byte_size(bytes)} >>\nstream\n", bytes, "\nendstream"]
end

MacosElixirArchiveSmoke.run(System.argv())
