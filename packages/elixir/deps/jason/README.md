# Jason [![Hex Version](https://img.shields.io/hexpm/v/jason.svg)](https://hex.pm/packages/jason) [![Hex Docs](https://img.shields.io/badge/docs-hexpm-blue.svg)](https://hexdocs.pm/jason/)

A blazing fast JSON parser and generator in pure Elixir.

The parser and generator are at least twice as fast as other Elixir/Erlang libraries
(most notably `Poison`).
The performance is comparable to `jiffy`, which is implemented in C as a NIF.
Jason is usually only twice as slow.

Both parser and generator fully conform to
[RFC 8259](https://tools.ietf.org/html/rfc8259) and
[ECMA 404](https://ecma-international.org/publications-and-standards/standards/ecma-404/)
standards. The parser is tested using [JSONTestSuite](https://github.com/nst/JSONTestSuite).

## Installation

The package can be installed by adding `jason` to your list of dependencies
in `mix.exs`:

```elixir
def deps do
  [{:jason, "~> 1.4"}]
end
```

## Basic Usage

``` elixir
iex(1)> Jason.encode!(%{"age" => 44, "name" => "Steve Irwin", "nationality" => "Australian"})
"{\"age\":44,\"name\":\"Steve Irwin\",\"nationality\":\"Australian\"}"

iex(2)> Jason.decode!(~s({"age":44,"name":"Steve Irwin","nationality":"Australian"}))
%{"age" => 44, "name" => "Steve Irwin", "nationality" => "Australian"}
```

Full documentation can be found at [https://hexdocs.pm/jason](https://hexdocs.pm/jason).

## Use with other libraries

### Postgrex

Versions starting at 0.14.0 use `Jason` by default. For earlier versions, please refer to
[previous versions of this document](https://github.com/michalmuskala/jason/tree/v1.1.2#postgrex).

### Ecto

Versions starting at 3.0.0 use `Jason` by default. For earlier versions, please refer to
[previous versions of this document](https://github.com/michalmuskala/jason/tree/v1.1.2#ecto).

### Plug (and Phoenix)

Phoenix starting at 1.4.0 uses `Jason` by default. For earlier versions, please refer to
[previous versions of this document](https://github.com/michalmuskala/jason/tree/v1.1.2#plug-and-phoenix).

### Absinthe

You need to pass the `:json_codec` option to `Absinthe.Plug`

```elixir
# When called directly:
plug Absinthe.Plug,
  schema: MyApp.Schema,
  json_codec: Jason

# When used in phoenix router:
forward "/api",
  to: Absinthe.Plug,
  init_opts: [schema: MyApp.Schema, json_codec: Jason]
```

## Benchmarks

Detailed benchmarks (including memory measurements):
https://gist.github.com/michalmuskala/4d64a5a7696ca84ac7c169a0206640d5

HTML reports for the benchmark (only performance measurements):
http://michal.muskala.eu/jason/decode.html and http://michal.muskala.eu/jason/encode.html

### Running

Benchmarks against most popular Elixir & Erlang json libraries can be executed after
going into the `bench/` folder and then executing `mix bench.encode` and `mix bench.decode`.
A HTML report of the benchmarks (after their execution) can be found in
`bench/output/encode.html` and `bench/output/decode.html` respectively.

## Differences to Poison

Jason has a couple feature differences compared to Poison.

  * Jason follows the JSON spec more strictly, for example it does not allow
    unescaped newline characters in JSON strings - e.g. `"\"\n\""` will
    produce a decoding error.
  * no support for decoding into data structures (the `as:` option).
  * no built-in encoders for `MapSet`, `Range` and `Stream`.
  * no support for encoding arbitrary structs - explicit implementation
    of the `Jason.Encoder` protocol is always required.
  * different pretty-printing customisation options (default `pretty: true` works the same)

### Encoders

If you require encoders for any of the unsupported collection types, I suggest
adding the needed implementations directly to your project:

```elixir
defimpl Jason.Encoder, for: [MapSet, Range, Stream] do
  def encode(struct, opts) do
    Jason.Encode.list(Enum.to_list(struct), opts)
  end
end
```

If you need to encode some struct that does not implement the protocol,
if you own the struct, you can derive the implementation specifying
which fields should be encoded to JSON:

```elixir
@derive {Jason.Encoder, only: [....]}
defstruct # ...
```

It is also possible to encode all fields, although this should be
used carefully to avoid accidentally leaking private information
when new fields are added:

```elixir
@derive Jason.Encoder
defstruct # ...
```

Finally, if you don't own the struct you want to encode to JSON,
you may use `Protocol.derive/3` placed outside of any module:

```elixir
Protocol.derive(Jason.Encoder, NameOfTheStruct, only: [...])
Protocol.derive(Jason.Encoder, NameOfTheStruct)
```

## Injecting an already encoded JSON inside a to-be-encoded structure

If parts of the to-be-encoded structure are already JSON-encoded, you can
use `Jason.Fragment` to mark the parts as already encoded, and avoid a
decoding/encoding roundtrip.

```elixir
already_encoded_json = Jason.encode!(%{hello: "world"})
Jason.encode!(%{foo: Jason.Fragment.new(already_encoded_json)})
```

This feature is especially useful if you need to cache a part of the JSON,
or if it is already provided by another system (e.g. `jsonb_agg` with Postgres).

## License

Jason is released under the Apache License 2.0 - see the [LICENSE](LICENSE) file.

Some elements of tests and benchmarks have their origins in the
[Poison library](https://github.com/devinus/poison) and were initially licensed under [CC0-1.0](https://creativecommons.org/publicdomain/zero/1.0/).
