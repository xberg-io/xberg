# xberg — Crystal Bindings

Document intelligence library (extraction, OCR, embeddings) — Crystal FFI bindings.

## Prerequisites

- **Crystal** >= 1.0.0
- **Rust** toolchain (to build the native FFI library)
- **System libraries**: `libheif >= 1.21` (for HEIF image support)

## Setting up libheif

If libheif is installed in a non-standard location (e.g., `/usr/local`),
set `PKG_CONFIG_PATH` when building the FFI library:

```bash
export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig
```

Verify with:
```bash
pkg-config --modversion libheif
# Should print 1.21 or higher
```

## Building

The native FFI library must be built before using the Crystal bindings.
A Makefile is provided at `packages/crystal/Makefile`:

```bash
cd packages/crystal
make                  # build everything (may need PKG_CONFIG_PATH set)
make ffi              # build just the native FFI library
make crystal          # build the Crystal module
make example          # build the example binary
make run              # build and run the example
```

If libheif is in `/usr/local`, pass the pkg-config path:

```bash
PKG_CONFIG_PATH=/usr/local/lib/pkgconfig make ffi
```

## Linking in your own project

```bash
crystal build my_program.cr \
  --link-flags="-L/path/to/target/release -Wl,-rpath,/path/to/target/release"
```

Or install the library system-wide:

```bash
sudo make install
crystal build my_program.cr   # no --link-flags needed
```

## Usage

```crystal
require "xberg"

# Create extraction config (only override what you need)
config = Xberg::ExtractionConfig.from_json(%({"force_ocr":true}))

# Extract from a file
input = Xberg::ExtractInput.from_json(
  %({"kind":"Uri","uri":"document.pdf"})
)
result = Xberg.extract(input, config)
puts result.results.size
result.results.each { |r| puts r.content[0..199] }
```

All bool, integer, string, array, and hash fields have sensible defaults —
only non-default enum/struct values need to be specified.

## Adding to shard.yml

```yaml
dependencies:
  xberg:
    github: xberg-io/xberg
```

## License

MIT — see [LICENSE](https://github.com/xberg-io/xberg/blob/main/LICENSE).
