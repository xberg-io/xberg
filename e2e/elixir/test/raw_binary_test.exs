defmodule E2e.RawBinaryTest do
  use ExUnit.Case, async: true

  @text "Raw binary extraction"

  test "encodes every byte value without treating binary data as text" do
    bytes = :erlang.list_to_binary(Enum.to_list(0..255))

    assert %{"kind" => "bytes", "bytes" => octets, "mime_type" => "text/plain"} =
             bytes |> input() |> Jason.encode!() |> Jason.decode!()

    assert octets == Enum.to_list(0..255)

    assert %{"kind" => "uri", "uri" => "file:///example.txt"} ==
             %Xberg.ExtractInput{uri: "file:///example.txt"} |> Jason.encode!() |> Jason.decode!()
  end

  test "extracts UTF-8 and non-UTF-8 typed inputs through extract/1" do
    assert {:ok, %{results: [%{content: @text, mime_type: "text/plain"}]}} =
             Xberg.extract(input: input(@text))

    assert {:ok, %{results: [%{content: content, mime_type: "text/plain"}]}} =
             Xberg.extract(input: input(arbitrary_bytes()))

    assert_non_utf8_text(content)
  end

  test "extracts mixed typed byte inputs through both batch wrappers" do
    inputs = [input(@text), input(arbitrary_bytes())]
    with_options = Enum.map(inputs, &%Xberg.ExtractInputWithOptions{input: &1})

    for result <- [
          Xberg.extract_batch(inputs: inputs),
          Xberg.extract_batch_with_input_options(inputs: with_options)
        ] do
      assert {:ok, %{results: [%{content: @text}, %{content: content}]}} = result
      assert_non_utf8_text(content)
    end
  end

  defp input(bytes), do: %Xberg.ExtractInput{kind: :bytes, bytes: bytes, mime_type: "text/plain"}

  defp arbitrary_bytes, do: @text <> <<0xFF>> <> " complete"

  defp assert_non_utf8_text(content) do
    assert String.starts_with?(content, @text)
    assert String.ends_with?(content, " complete")
  end
end
