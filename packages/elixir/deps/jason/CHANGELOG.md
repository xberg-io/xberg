# Changelog

## 1.4.5 (05.05.2026)

* Add support for Decimal 3.0

## 1.4.4 (26.07.2024)

* Fix warnings on Elixir 1.17 by conditionally compiling Decimal support

## 1.4.3 (29.06.2024)

* Fix derive with _ struct key

## 1.4.2 (29.06.2024)

* Fix compiler warnings for Elixir 1.17

## 1.4.1 (06.07.2023)

* Add limit to decoded integer sizes of 1024 digits. This can be changed
  with the `decoding_integer_digit_limit` app env config.

## 1.4.0 (12.09.2022)

### Enhancements

* Use the `:erlang.float_to_binary(_, [:short])` function, instead of `io_lib_format.fwrite_g/1`
  where available (OTP 24.1+). This provides equivalent output with much less memory used
  and significantly improved performance.

## 1.3.0 (21.12.2021)

### Enhancements

* Add the `Jason.OrderedObject` struct
* Support decoding objects preserving all the keys with `objects: :ordered_objects` option
* Support decoding floats to `Decimal` with `floats: :decimals` option
* Add `~j` and `~J` sigils in module `Jason.Sigil` to support writing JSON literals in code

### Fixes
* Fix error reporting when decoding strings (it was possible to mis-attribute the offending byte)
* Verify fields given to `@derive`

## 1.2.2 (08.09.2020)

### Enhancements

* Support Decimal 2.0

## 1.2.1 (04.05.2020)

### Security

* Fix `html_safe` escaping in `Jason.encode`

  The `<!--` sequence of characters would not be escaped in `Jason.encode`
  with`html_escape` mode, which could lead to DoS attacks when used for
  embedding of arbitrary, user controlled strings into HTML through JSON
  (e.g. inside of `<script>` tags).

  If you were not using the `html_safe` option, you are not affected.

  Affected versions: < 1.2.1
  Patched versions: >= 1.2.1

## 1.2.0 (17.03.2020)

### Enhancements

* Add `Jason.Encode.keyword/2`
  ([cb1f26a](https://github.com/michalmuskala/jason/commit/cb1f26a)).

### Bug fixes

* Fix `Jason.Helpers.json_map/1` value expansion
  ([70b046a](https://github.com/michalmuskala/jason/commit/70b046a)).

## 1.1.2 (19.10.2018)

### Bug fixes

* correctly handle the `pretty: false` option
  ([ba318c8](https://github.com/michalmuskala/jason/commit/ba318c8)).

## 1.1.1 (10.07.2018)

### Bug fixes

* correctly handle escape sequences in strings when pretty printing
  ([794bbe4](https://github.com/michalmuskala/jason/commit/794bbe4)).

## 1.1.0 (02.07.2018)

### Enhancements

* pretty-printing support through `Jason.Formatter` and `pretty: true` option
  in `Jason.encode/2` ([d758e36](https://github.com/michalmuskala/jason/commit/d758e36)).

### Bug fixes

* silence variable warnings for fields with underscores used during deriving
  ([88dd85c](https://github.com/michalmuskala/jason/commit/88dd85c)).
* **potential incompatibility** don't raise `Protocol.UndefinedError` in non-bang functions
  ([ad0f57b](https://github.com/michalmuskala/jason/commit/ad0f57b)).

## 1.0.1 (02.07.2018)

### Bug fixes

* fix `Jason.Encode.escape` type ([a57b430](https://github.com/michalmuskala/jason/commit/a57b430))
* multiple documentation improvements

## 1.0.0 (26.01.2018)

No changes

## 1.0.0-rc.3 (26.01.2018)

### Changes

* update `escape` option of `Jason.encode/2` to take values:
  `:json | :unicode_safe | :html_safe | :javascript_safe` for consistency. Old values of
  `:unicode` and `:javascript` are still supported for compatibility with Poison.
  ([f42dcbd](https://github.com/michalmuskala/jason/commit/f42dcbd))

## 1.0.0-rc.2 (07.01.2018)

### Bug fixes

* add type for `strings` option ([b459ee4](https://github.com/michalmuskala/jason/commit/b459ee4))
* support iodata in `decode!` ([a1f3456](https://github.com/michalmuskala/jason/commit/a1f3456))

## 1.0.0-rc.1 (22.12.2017)

* Initial release
