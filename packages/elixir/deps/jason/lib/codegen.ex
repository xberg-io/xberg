defmodule Jason.Codegen do
  @moduledoc false

  alias Jason.{Encode, EncodeError}

  def jump_table(ranges, default) do
    ranges
    |> ranges_to_orddict()
    |> :array.from_orddict(default)
    |> :array.to_orddict()
  end

  def jump_table(ranges, default, max) do
    ranges
    |> ranges_to_orddict()
    |> :array.from_orddict(default)
    |> resize(max)
    |> :array.to_orddict()
  end

  defmacro bytecase(var, do: clauses) do
    {ranges, default, literals} = clauses_to_ranges(clauses, [], __CALLER__)

    jump_table = jump_table(ranges, default)

    quote do
      case unquote(var) do
        unquote(jump_table_to_clauses(jump_table, literals))
      end
    end
  end

  defmacro bytecase(var, max, do: clauses) do
    {ranges, default, empty} = clauses_to_ranges(clauses, [], __CALLER__)

    jump_table = jump_table(ranges, default, max)

    quote do
      case unquote(var) do
        unquote(jump_table_to_clauses(jump_table, empty))
      end
    end
  end

  def build_kv_iodata(kv, encode_args) do
    elements =
      kv
      |> Enum.map(&encode_pair(&1, encode_args))
      |> Enum.intersperse(",")

    collapse_static(List.flatten(["{", elements] ++ ~c'}'))
  end

  defp clauses_to_ranges([{:->, _, [[{:in, _, [byte, range]}, rest], action]} | tail], acc, env) do
    range = Macro.expand(range, env)
    clauses_to_ranges(tail, [{range, {byte, rest, action}} | acc], env)
  end

  defp clauses_to_ranges([{:->, _, [[default, rest], action]} | tail], acc, _env) do
    {Enum.reverse(acc), {default, rest, action}, literal_clauses(tail)}
  end

  defp literal_clauses(clauses) do
    Enum.map(clauses, fn {:->, _, [[literal], action]} ->
      {literal, action}
    end)
  end

  defp jump_table_to_clauses([{val, {{:_, _, _}, rest, action}} | tail], empty) do
    quote do
      <<unquote(val), unquote(rest)::bits>> ->
        unquote(action)
    end ++ jump_table_to_clauses(tail, empty)
  end

  defp jump_table_to_clauses([{val, {byte, rest, action}} | tail], empty) do
    quote do
      <<unquote(byte), unquote(rest)::bits>> when unquote(byte) === unquote(val) ->
        unquote(action)
    end ++ jump_table_to_clauses(tail, empty)
  end

  defp jump_table_to_clauses([], literals) do
    Enum.flat_map(literals, fn {pattern, action} ->
      quote do
        unquote(pattern) ->
          unquote(action)
      end
    end)
  end

  defp resize(array, size), do: :array.resize(size, array)

  defp ranges_to_orddict(ranges) do
    ranges
    |> Enum.flat_map(fn
         {int, value} when is_integer(int) ->
           [{int, value}]

         {enum, value} ->
           Enum.map(enum, &{&1, value})
       end)
    |> :orddict.from_list()
  end

  defp encode_pair({key, value}, encode_args) do
    key = IO.iodata_to_binary(Encode.key(key, &escape_key/1))
    key = "\"" <> key <> "\":"
    [key, quote(do: Encode.value(unquote(value), unquote_splicing(encode_args)))]
  end

  defp escape_key(binary) do
    check_safe_key!(binary)
    binary
  end

  defp check_safe_key!(binary) do
    for <<(<<byte>> <- binary)>> do
      if byte > 0x7F or byte < 0x1F or byte in ~c'"\\/' do
        raise EncodeError,
              "invalid byte #{inspect(byte, base: :hex)} in literal key: #{inspect(binary)}"
      end
    end

    :ok
  end

  defp collapse_static([bin1, bin2 | rest]) when is_binary(bin1) and is_binary(bin2) do
    collapse_static([bin1 <> bin2 | rest])
  end

  defp collapse_static([other | rest]) do
    [other | collapse_static(rest)]
  end

  defp collapse_static([]) do
    []
  end
end
