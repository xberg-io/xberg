defmodule Jason.EncodeError do
  defexception [:message]

  @type t :: %__MODULE__{message: String.t}

  def new({:duplicate_key, key}) do
    %__MODULE__{message: "duplicate key: #{key}"}
  end
  def new({:invalid_byte, byte, original}) do
    %__MODULE__{message: "invalid byte #{inspect byte, base: :hex} in #{inspect original}"}
  end
end

defmodule Jason.Encode do
  @moduledoc """
  Utilities for encoding elixir values to JSON.
  """

  import Bitwise

  alias Jason.{Codegen, EncodeError, Encoder, Fragment, OrderedObject}

  @typep escape :: (String.t -> iodata)
  @typep encode_map :: (map, escape, encode_map -> iodata)
  @opaque opts :: {escape, encode_map}

  @dialyzer :no_improper_lists

  # @compile :native

  @doc false
  @spec encode(any, map) :: {:ok, iodata} | {:error, EncodeError.t | Exception.t}
  def encode(value, opts) do
    escape = escape_function(opts)
    encode_map = encode_map_function(opts)
    try do
      {:ok, value(value, escape, encode_map)}
    catch
      :throw, %EncodeError{} = e ->
        {:error, e}
      :error, %Protocol.UndefinedError{protocol: Jason.Encoder} = e ->
        {:error, e}
    end
  end

  defp encode_map_function(%{maps: maps}) do
    case maps do
      :naive -> &map_naive/3
      :strict -> &map_strict/3
    end
  end

  defp escape_function(%{escape: escape}) do
    case escape do
      :json -> &escape_json/1
      :html_safe -> &escape_html/1
      :unicode_safe -> &escape_unicode/1
      :javascript_safe -> &escape_javascript/1
      # Keep for compatibility with Poison
      :javascript -> &escape_javascript/1
      :unicode -> &escape_unicode/1
    end
  end

  @doc """
  Equivalent to calling the `Jason.Encoder.encode/2` protocol function.

  Slightly more efficient for built-in types because of the internal dispatching.
  """
  @spec value(term, opts) :: iodata
  def value(value, {escape, encode_map}) do
    value(value, escape, encode_map)
  end

  @doc false
  # We use this directly in the helpers and deriving for extra speed
  def value(value, escape, _encode_map) when is_atom(value) do
    encode_atom(value, escape)
  end

  def value(value, escape, _encode_map) when is_binary(value) do
    encode_string(value, escape)
  end

  def value(value, _escape, _encode_map) when is_integer(value) do
    integer(value)
  end

  def value(value, _escape, _encode_map) when is_float(value) do
    float(value)
  end

  def value(value, escape, encode_map) when is_list(value) do
    list(value, escape, encode_map)
  end

  def value(%{__struct__: module} = value, escape, encode_map) do
    struct(value, escape, encode_map, module)
  end

  def value(value, escape, encode_map) when is_map(value) do
    case Map.to_list(value) do
      [] -> "{}"
      keyword -> encode_map.(keyword, escape, encode_map)
    end
  end

  def value(value, escape, encode_map) do
    Encoder.encode(value, {escape, encode_map})
  end

  @compile {:inline, integer: 1, float: 1}

  @spec atom(atom, opts) :: iodata
  def atom(atom, {escape, _encode_map}) do
    encode_atom(atom, escape)
  end

  defp encode_atom(nil, _escape), do: "null"
  defp encode_atom(true, _escape), do: "true"
  defp encode_atom(false, _escape), do: "false"
  defp encode_atom(atom, escape),
    do: encode_string(Atom.to_string(atom), escape)

  @spec integer(integer) :: iodata
  def integer(integer) do
    Integer.to_string(integer)
  end

  has_short_format = try do
    :erlang.float_to_binary(1.0, [:short])
  catch
    _, _ -> false
  else
    _ -> true
  end

  @spec float(float) :: iodata
  if has_short_format do
    def float(float) do
      :erlang.float_to_binary(float, [:short])
    end
  else
    def float(float) do
      :io_lib_format.fwrite_g(float)
   end
  end

  @spec list(list, opts) :: iodata
  def list(list, {escape, encode_map}) do
    list(list, escape, encode_map)
  end

  defp list([], _escape, _encode_map) do
    "[]"
  end

  defp list([head | tail], escape, encode_map) do
    [?[, value(head, escape, encode_map)
     | list_loop(tail, escape, encode_map)]
  end

  defp list_loop([], _escape, _encode_map) do
    ~c']'
  end

  defp list_loop([head | tail], escape, encode_map) do
    [?,, value(head, escape, encode_map)
     | list_loop(tail, escape, encode_map)]
  end

  @spec keyword(keyword, opts) :: iodata
  def keyword(list, _) when list == [], do: "{}"
  def keyword(list, {escape, encode_map}) when is_list(list) do
    encode_map.(list, escape, encode_map)
  end

  @spec map(map, opts) :: iodata
  def map(value, {escape, encode_map}) do
    case Map.to_list(value) do
      [] -> "{}"
      keyword -> encode_map.(keyword, escape, encode_map)
    end
  end

  defp map_naive([{key, value} | tail], escape, encode_map) do
    ["{\"", key(key, escape), "\":",
    value(value, escape, encode_map)
    | map_naive_loop(tail, escape, encode_map)]
  end

  defp map_naive_loop([], _escape, _encode_map) do
    ~c'}'
  end

  defp map_naive_loop([{key, value} | tail], escape, encode_map) do
    [",\"", key(key, escape), "\":",
     value(value, escape, encode_map)
     | map_naive_loop(tail, escape, encode_map)]
  end

  defp map_strict([{key, value} | tail], escape, encode_map) do
    key = IO.iodata_to_binary(key(key, escape))
    visited = %{key => []}
    ["{\"", key, "\":",
     value(value, escape, encode_map)
     | map_strict_loop(tail, escape, encode_map, visited)]
  end

  defp map_strict_loop([], _encode_map, _escape, _visited) do
    ~c'}'
  end

  defp map_strict_loop([{key, value} | tail], escape, encode_map, visited) do
    key = IO.iodata_to_binary(key(key, escape))
    case visited do
      %{^key => _} ->
        error({:duplicate_key, key})
      _ ->
        visited = Map.put(visited, key, [])
        [",\"", key, "\":",
         value(value, escape, encode_map)
         | map_strict_loop(tail, escape, encode_map, visited)]
    end
  end

  @spec struct(struct, opts) :: iodata
  def struct(%module{} = value, {escape, encode_map}) do
    struct(value, escape, encode_map, module)
  end

  # TODO: benchmark the effect of inlining the to_iso8601 functions
  for module <- [Date, Time, NaiveDateTime, DateTime] do
    defp struct(value, _escape, _encode_map, unquote(module)) do
      [?", unquote(module).to_iso8601(value), ?"]
    end
  end

  if Code.ensure_loaded?(Decimal) do
    defp struct(value, _escape, _encode_map, Decimal) do
      [?", Decimal.to_string(value, :normal), ?"]
    end
  end

  defp struct(value, escape, encode_map, Fragment) do
    %{encode: encode} = value
    encode.({escape, encode_map})
  end

  defp struct(value, escape, encode_map, OrderedObject) do
    case value do
      %{values: []} -> "{}"
      %{values: values} -> encode_map.(values, escape, encode_map)
    end
  end

  defp struct(value, escape, encode_map, _module) do
    Encoder.encode(value, {escape, encode_map})
  end

  @doc false
  # This is used in the helpers and deriving implementation
  def key(string, escape) when is_binary(string) do
    escape.(string)
  end
  def key(atom, escape) when is_atom(atom) do
    string = Atom.to_string(atom)
    escape.(string)
  end
  def key(other, escape) do
    string = String.Chars.to_string(other)
    escape.(string)
  end

  @spec string(String.t, opts) :: iodata
  def string(string, {escape, _encode_map}) do
    encode_string(string, escape)
  end

  defp encode_string(string, escape) do
    [?", escape.(string), ?"]
  end

  slash_escapes = Enum.zip(~c'\b\t\n\f\r\"\\', ~c'btnfr"\\')
  surogate_escapes = Enum.zip([0x2028, 0x2029], ["\\u2028", "\\u2029"])
  ranges = [{0x00..0x1F, :unicode} | slash_escapes]
  html_ranges = [{0x00..0x1F, :unicode}, {?<, :unicode}, {?/, ?/} | slash_escapes]
  escape_jt = Codegen.jump_table(html_ranges, :error)

  Enum.each(escape_jt, fn
    {byte, :unicode} ->
      sequence = List.to_string(:io_lib.format("\\u~4.16.0B", [byte]))
      defp escape(unquote(byte)), do: unquote(sequence)
    {byte, char} when is_integer(char) ->
      defp escape(unquote(byte)), do: unquote(<<?\\, char>>)
    {byte, :error} ->
      defp escape(unquote(byte)), do: throw(:error)
  end)

  ## regular JSON escape

  json_jt = Codegen.jump_table(ranges, :chunk, 0x7F + 1)

  defp escape_json(data) do
    escape_json(data, [], data, 0)
  end

  Enum.map(json_jt, fn
    {byte, :chunk} ->
      defp escape_json(<<byte, rest::bits>>, acc, original, skip)
           when byte === unquote(byte) do
        escape_json_chunk(rest, acc, original, skip, 1)
      end
    {byte, _escape} ->
      defp escape_json(<<byte, rest::bits>>, acc, original, skip)
           when byte === unquote(byte) do
        acc = [acc | escape(byte)]
        escape_json(rest, acc, original, skip + 1)
      end
  end)
  defp escape_json(<<char::utf8, rest::bits>>, acc, original, skip)
       when char <= 0x7FF do
    escape_json_chunk(rest, acc, original, skip, 2)
  end
  defp escape_json(<<char::utf8, rest::bits>>, acc, original, skip)
       when char <= 0xFFFF do
    escape_json_chunk(rest, acc, original, skip, 3)
  end
  defp escape_json(<<_char::utf8, rest::bits>>, acc, original, skip) do
    escape_json_chunk(rest, acc, original, skip, 4)
  end
  defp escape_json(<<>>, acc, _original, _skip) do
    acc
  end
  defp escape_json(<<byte, _rest::bits>>, _acc, original, _skip) do
    error({:invalid_byte, byte, original})
  end

  Enum.map(json_jt, fn
    {byte, :chunk} ->
      defp escape_json_chunk(<<byte, rest::bits>>, acc, original, skip, len)
           when byte === unquote(byte) do
        escape_json_chunk(rest, acc, original, skip, len + 1)
      end
    {byte, _escape} ->
      defp escape_json_chunk(<<byte, rest::bits>>, acc, original, skip, len)
           when byte === unquote(byte) do
        part = binary_part(original, skip, len)
        acc = [acc, part | escape(byte)]
        escape_json(rest, acc, original, skip + len + 1)
      end
  end)
  defp escape_json_chunk(<<char::utf8, rest::bits>>, acc, original, skip, len)
       when char <= 0x7FF do
    escape_json_chunk(rest, acc, original, skip, len + 2)
  end
  defp escape_json_chunk(<<char::utf8, rest::bits>>, acc, original, skip, len)
       when char <= 0xFFFF do
    escape_json_chunk(rest, acc, original, skip, len + 3)
  end
  defp escape_json_chunk(<<_char::utf8, rest::bits>>, acc, original, skip, len) do
    escape_json_chunk(rest, acc, original, skip, len + 4)
  end
  defp escape_json_chunk(<<>>, acc, original, skip, len) do
    part = binary_part(original, skip, len)
    [acc | part]
  end
  defp escape_json_chunk(<<byte, _rest::bits>>, _acc, original, _skip, _len) do
    error({:invalid_byte, byte, original})
  end

  ## javascript safe JSON escape

  defp escape_javascript(data) do
    escape_javascript(data, [], data, 0)
  end

  Enum.map(json_jt, fn
    {byte, :chunk} ->
      defp escape_javascript(<<byte, rest::bits>>, acc, original, skip)
            when byte === unquote(byte) do
        escape_javascript_chunk(rest, acc, original, skip, 1)
      end
    {byte, _escape} ->
      defp escape_javascript(<<byte, rest::bits>>, acc, original, skip)
            when byte === unquote(byte) do
        acc = [acc | escape(byte)]
        escape_javascript(rest, acc, original, skip + 1)
      end
  end)
  defp escape_javascript(<<char::utf8, rest::bits>>, acc, original, skip)
        when char <= 0x7FF do
    escape_javascript_chunk(rest, acc, original, skip, 2)
  end
  Enum.map(surogate_escapes, fn {byte, escape} ->
    defp escape_javascript(<<unquote(byte)::utf8, rest::bits>>, acc, original, skip) do
      acc = [acc | unquote(escape)]
      escape_javascript(rest, acc, original, skip + 3)
    end
  end)
  defp escape_javascript(<<char::utf8, rest::bits>>, acc, original, skip)
        when char <= 0xFFFF do
    escape_javascript_chunk(rest, acc, original, skip, 3)
  end
  defp escape_javascript(<<_char::utf8, rest::bits>>, acc, original, skip) do
    escape_javascript_chunk(rest, acc, original, skip, 4)
  end
  defp escape_javascript(<<>>, acc, _original, _skip) do
    acc
  end
  defp escape_javascript(<<byte, _rest::bits>>, _acc, original, _skip) do
    error({:invalid_byte, byte, original})
  end

  Enum.map(json_jt, fn
    {byte, :chunk} ->
      defp escape_javascript_chunk(<<byte, rest::bits>>, acc, original, skip, len)
            when byte === unquote(byte) do
        escape_javascript_chunk(rest, acc, original, skip, len + 1)
      end
    {byte, _escape} ->
      defp escape_javascript_chunk(<<byte, rest::bits>>, acc, original, skip, len)
            when byte === unquote(byte) do
        part = binary_part(original, skip, len)
        acc = [acc, part | escape(byte)]
        escape_javascript(rest, acc, original, skip + len + 1)
      end
  end)
  defp escape_javascript_chunk(<<char::utf8, rest::bits>>, acc, original, skip, len)
        when char <= 0x7FF do
    escape_javascript_chunk(rest, acc, original, skip, len + 2)
  end
  Enum.map(surogate_escapes, fn {byte, escape} ->
    defp escape_javascript_chunk(<<unquote(byte)::utf8, rest::bits>>, acc, original, skip, len) do
      part = binary_part(original, skip, len)
      acc = [acc, part | unquote(escape)]
      escape_javascript(rest, acc, original, skip + len + 3)
    end
  end)
  defp escape_javascript_chunk(<<char::utf8, rest::bits>>, acc, original, skip, len)
        when char <= 0xFFFF do
    escape_javascript_chunk(rest, acc, original, skip, len + 3)
  end
  defp escape_javascript_chunk(<<_char::utf8, rest::bits>>, acc, original, skip, len) do
    escape_javascript_chunk(rest, acc, original, skip, len + 4)
  end
  defp escape_javascript_chunk(<<>>, acc, original, skip, len) do
    part = binary_part(original, skip, len)
    [acc | part]
  end
  defp escape_javascript_chunk(<<byte, _rest::bits>>, _acc, original, _skip, _len) do
    error({:invalid_byte, byte, original})
  end

  ## HTML safe JSON escape

  html_jt = Codegen.jump_table(html_ranges, :chunk, 0x7F + 1)

  defp escape_html(data) do
    escape_html(data, [], data, 0)
  end

  Enum.map(html_jt, fn
    {byte, :chunk} ->
      defp escape_html(<<byte, rest::bits>>, acc, original, skip)
            when byte === unquote(byte) do
        escape_html_chunk(rest, acc, original, skip, 1)
      end
    {byte, _escape} ->
      defp escape_html(<<byte, rest::bits>>, acc, original, skip)
            when byte === unquote(byte) do
        acc = [acc | escape(byte)]
        escape_html(rest, acc, original, skip + 1)
      end
  end)
  defp escape_html(<<char::utf8, rest::bits>>, acc, original, skip)
        when char <= 0x7FF do
    escape_html_chunk(rest, acc, original, skip, 2)
  end
  Enum.map(surogate_escapes, fn {byte, escape} ->
    defp escape_html(<<unquote(byte)::utf8, rest::bits>>, acc, original, skip) do
      acc = [acc | unquote(escape)]
      escape_html(rest, acc, original, skip + 3)
    end
  end)
  defp escape_html(<<char::utf8, rest::bits>>, acc, original, skip)
        when char <= 0xFFFF do
    escape_html_chunk(rest, acc, original, skip, 3)
  end
  defp escape_html(<<_char::utf8, rest::bits>>, acc, original, skip) do
    escape_html_chunk(rest, acc, original, skip, 4)
  end
  defp escape_html(<<>>, acc, _original, _skip) do
    acc
  end
  defp escape_html(<<byte, _rest::bits>>, _acc, original, _skip) do
    error({:invalid_byte, byte, original})
  end

  Enum.map(html_jt, fn
    {byte, :chunk} ->
      defp escape_html_chunk(<<byte, rest::bits>>, acc, original, skip, len)
            when byte === unquote(byte) do
        escape_html_chunk(rest, acc, original, skip, len + 1)
      end
    {byte, _escape} ->
      defp escape_html_chunk(<<byte, rest::bits>>, acc, original, skip, len)
            when byte === unquote(byte) do
        part = binary_part(original, skip, len)
        acc = [acc, part | escape(byte)]
        escape_html(rest, acc, original, skip + len + 1)
      end
  end)
  defp escape_html_chunk(<<char::utf8, rest::bits>>, acc, original, skip, len)
        when char <= 0x7FF do
    escape_html_chunk(rest, acc, original, skip, len + 2)
  end
  Enum.map(surogate_escapes, fn {byte, escape} ->
    defp escape_html_chunk(<<unquote(byte)::utf8, rest::bits>>, acc, original, skip, len) do
      part = binary_part(original, skip, len)
      acc = [acc, part | unquote(escape)]
      escape_html(rest, acc, original, skip + len + 3)
    end
  end)
  defp escape_html_chunk(<<char::utf8, rest::bits>>, acc, original, skip, len)
        when char <= 0xFFFF do
    escape_html_chunk(rest, acc, original, skip, len + 3)
  end
  defp escape_html_chunk(<<_char::utf8, rest::bits>>, acc, original, skip, len) do
    escape_html_chunk(rest, acc, original, skip, len + 4)
  end
  defp escape_html_chunk(<<>>, acc, original, skip, len) do
    part = binary_part(original, skip, len)
    [acc | part]
  end
  defp escape_html_chunk(<<byte, _rest::bits>>, _acc, original, _skip, _len) do
    error({:invalid_byte, byte, original})
  end

  ## unicode escape

  defp escape_unicode(data) do
    escape_unicode(data, [], data, 0)
  end

  Enum.map(json_jt, fn
    {byte, :chunk} ->
      defp escape_unicode(<<byte, rest::bits>>, acc, original, skip)
           when byte === unquote(byte) do
        escape_unicode_chunk(rest, acc, original, skip, 1)
      end
    {byte, _escape} ->
      defp escape_unicode(<<byte, rest::bits>>, acc, original, skip)
           when byte === unquote(byte) do
        acc = [acc | escape(byte)]
        escape_unicode(rest, acc, original, skip + 1)
      end
  end)
  defp escape_unicode(<<char::utf8, rest::bits>>, acc, original, skip)
       when char <= 0xFF do
    acc = [acc, "\\u00" | Integer.to_string(char, 16)]
    escape_unicode(rest, acc, original, skip + 2)
  end
  defp escape_unicode(<<char::utf8, rest::bits>>, acc, original, skip)
        when char <= 0x7FF do
    acc = [acc, "\\u0" | Integer.to_string(char, 16)]
    escape_unicode(rest, acc, original, skip + 2)
  end
  defp escape_unicode(<<char::utf8, rest::bits>>, acc, original, skip)
        when char <= 0xFFF do
    acc = [acc, "\\u0" | Integer.to_string(char, 16)]
    escape_unicode(rest, acc, original, skip + 3)
  end
  defp escape_unicode(<<char::utf8, rest::bits>>, acc, original, skip)
        when char <= 0xFFFF do
    acc = [acc, "\\u" | Integer.to_string(char, 16)]
    escape_unicode(rest, acc, original, skip + 3)
  end
  defp escape_unicode(<<char::utf8, rest::bits>>, acc, original, skip) do
    char = char - 0x10000
    acc =
      [
        acc,
        "\\uD", Integer.to_string(0x800 ||| (char >>> 10), 16),
        "\\uD" | Integer.to_string(0xC00 ||| (char &&& 0x3FF), 16)
      ]
    escape_unicode(rest, acc, original, skip + 4)
  end
  defp escape_unicode(<<>>, acc, _original, _skip) do
    acc
  end
  defp escape_unicode(<<byte, _rest::bits>>, _acc, original, _skip) do
    error({:invalid_byte, byte, original})
  end

  Enum.map(json_jt, fn
    {byte, :chunk} ->
      defp escape_unicode_chunk(<<byte, rest::bits>>, acc, original, skip, len)
            when byte === unquote(byte) do
        escape_unicode_chunk(rest, acc, original, skip, len + 1)
      end
    {byte, _escape} ->
      defp escape_unicode_chunk(<<byte, rest::bits>>, acc, original, skip, len)
            when byte === unquote(byte) do
        part = binary_part(original, skip, len)
        acc = [acc, part | escape(byte)]
        escape_unicode(rest, acc, original, skip + len + 1)
      end
  end)
  defp escape_unicode_chunk(<<char::utf8, rest::bits>>, acc, original, skip, len)
       when char <= 0xFF do
    part = binary_part(original, skip, len)
    acc = [acc, part, "\\u00" | Integer.to_string(char, 16)]
    escape_unicode(rest, acc, original, skip + len + 2)
  end
  defp escape_unicode_chunk(<<char::utf8, rest::bits>>, acc, original, skip, len)
        when char <= 0x7FF do
    part = binary_part(original, skip, len)
    acc = [acc, part, "\\u0" | Integer.to_string(char, 16)]
    escape_unicode(rest, acc, original, skip + len + 2)
  end
  defp escape_unicode_chunk(<<char::utf8, rest::bits>>, acc, original, skip, len)
        when char <= 0xFFF do
    part = binary_part(original, skip, len)
    acc = [acc, part, "\\u0" | Integer.to_string(char, 16)]
    escape_unicode(rest, acc, original, skip + len + 3)
  end
  defp escape_unicode_chunk(<<char::utf8, rest::bits>>, acc, original, skip, len)
        when char <= 0xFFFF do
    part = binary_part(original, skip, len)
    acc = [acc, part, "\\u" | Integer.to_string(char, 16)]
    escape_unicode(rest, acc, original, skip + len + 3)
  end
  defp escape_unicode_chunk(<<char::utf8, rest::bits>>, acc, original, skip, len) do
    char = char - 0x10000
    part = binary_part(original, skip, len)
    acc =
      [
        acc, part,
        "\\uD", Integer.to_string(0x800 ||| (char >>> 10), 16),
        "\\uD" | Integer.to_string(0xC00 ||| (char &&& 0x3FF), 16)
      ]
    escape_unicode(rest, acc, original, skip + len + 4)
  end
  defp escape_unicode_chunk(<<>>, acc, original, skip, len) do
    part = binary_part(original, skip, len)
    [acc | part]
  end
  defp escape_unicode_chunk(<<byte, _rest::bits>>, _acc, original, _skip, _len) do
    error({:invalid_byte, byte, original})
  end

  @compile {:inline, error: 1}
  defp error(error) do
    throw EncodeError.new(error)
  end
end
