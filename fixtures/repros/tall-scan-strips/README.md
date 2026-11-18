# Tall scan strip regression

The fixtures contain generated grayscale scans with forty distinct text lines.
The tall and short rasters contain the same text. The mixed document puts a
native text page before the tall scan. The page size and resolution trigger
the render allocation cap only for the tall raster.

Regenerate with Pillow 13.0.0 and Arial Regular.

```sh
uv run --no-project --with pillow==13.0.0 python generate.py . /path/to/Arial.ttf
```

The bilevel fixture has the same 40 source lines in a 1275 × 12000 raster.
Each packed DeviceGray row ends with five padding bits.
Regenerate it with the bundled DejaVu Sans font:

```sh
uv run --no-project --with pillow==13.0.0 python generate.py . \
  ../../../crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf --bilevel-only
```

The neutral RGB variant stores the same 12,000-pixel grayscale raster in
three identical channels. Use the same font and `--neutral-rgb-only`.

The grayscale JPEG fixture uses the same 1275 × 12000 source grid and 150 DPI
as the bilevel fixture. It uses sequential 8-bit JPEG with quality 88 and
optimized entropy coding. Generate it with the bundled DejaVu Sans font and
Pillow 11.3.0.

```sh
uv run --no-project --with pillow==11.3.0 python generate.py . \
  ../../../crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf --jpeg-only
```

The mixed JPEG fixture puts the native page of the mixed document before the same
grayscale JPEG scan. Generate it with the same font and Pillow version and
`--jpeg-mixed-only`.

```sh
uv run --no-project --with pillow==11.3.0 python generate.py . \
  ../../../crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf --jpeg-mixed-only
```

The predictor and indexed fixtures use the same 40 lines at 20 px. The
DeviceGray predictor declares PNG Up and alternates actual None and Up row
tags. The one-bit Indexed image maps index zero to white and sets all five
padding bits on each row to the black index.

```sh
uv run --no-project --with pillow==11.3.0 python generate.py . \
  ../../../crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf --predictor-indexed-only
```

The 120 DPI variants (bilevel, neutral RGB, Indexed and grayscale JPEG) are rendered
at 1020 × 9600 px with the same 40 lines at 0.8 of the 150 DPI glyph size, on a
612 × 5760 pt page. Regenerate them with Pillow 13.0.0 and the bundled DejaVu Sans font:

```sh
uv run --no-project --with pillow==13.0.0 python generate.py . \
  ../../../crates/xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf --lower-density-only
```

Of these, only the Indexed scan reads back as all 440 source words; the others pin the 150 DPI floor.

## Source geometry at different densities

`large-text-150.pdf` and `large-text-300.pdf` contain the same physical scan on
216 × 5760 pt pages. The 450 × 12000 grayscale raster is enlarged by exact
2 × 2 pixel replication to 900 × 24000; no new glyphs or OCR-derived text
are introduced. A 64 px source line precedes forty 20 px prose lines.
`large-text-expected.txt` comes directly from the strings drawn by the generator.
The source font is the repository's DejaVu Sans, checked by SHA-256 in the script.

```sh
mise exec -- uv run --no-project --with pillow==13.0.0 python generate-scaled.py
```

The public API tests also losslessly compress the existing neutral RGB fixture's
page-paint stream, verifying identical decoded instructions before extraction.
An additional-drawing variant exercises refusal of pages that are not one scan.

Successful source-strip recovery retains the original rendered and processed frames
in `ocr_source_strip_original_frames`, ordered by accepted document page number.
Detached-stage page IDs are rebound to those document pages. The existing
`ocr_source_strip_original_frame` object remains a compatibility view of the first
entry. These historical dimensions survive both whole-document and automatic
per-page OCR projection; they are not the recovered text's current coordinate frame.
The public tests check both keys and the actual 154 × 4096 processed frame against
the retained native render dimensions at both source densities. They exercise raw, Flate and
ASCII85+Flate page-paint streams, including separately serialized extra-paint
controls that must report an uncertified source. The low-density geometry tests
retain the pre-existing 80-row inclusive and 48-column exclusive limits at 120 DPI.

## Integrated qualification

The release-feature run executes 38 synthetic tests covering encodings, DPI,
PSM, language, cache behavior and standalone image warnings. The complete run
adds ten tests for the public `sharable_web_guide.pdf`, including the paired
whole-guide, Page 5 and 8, right-column and annotation checks.

Both modes run offline. Provision ONNX Runtime as in Rust CI, then stage the
pinned RT-DETR model in the Hugging Face cache before invoking the task.
Use `-- synthetic` on the last line to run only the synthetic tests and skip
the two corpus commands.

```sh
(
  set -eu
  git submodule update --init test_documents
  python3 test_documents/scripts/fetch_corpus.py --include 'pdf/sharable_web_guide.pdf'

  scratch="$(mktemp -d)"
  trap 'rm -rf "$scratch"' EXIT
  export HF_HOME="$scratch/hf" HF_HUB_CACHE="$scratch/hub" HUGGINGFACE_HUB_CACHE="$scratch/hub"
  export XDG_CACHE_HOME="$scratch/xdg"
  revision=c6bf493e2f7b0b9a29a5870da9880c14e20ff0a3
  model="$HF_HUB_CACHE/models--xberg-io--layout-models/snapshots/$revision/rtdetr/model.onnx"
  mkdir -p "$(dirname "$model")"
  curl --fail --silent --show-error --location --retry 3 --connect-timeout 30 --max-time 600 \
    --proto '=https' --proto-redir '=https' \
    "https://huggingface.co/xberg-io/layout-models/resolve/$revision/rtdetr/model.onnx" \
    --output "$model"
  test "$(wc -c <"$model")" -eq 169089059
  printf '%s  %s\n' 3bf2fb0ee6df87435b7ae47f0f3930ec3dc97ec56fd824acc6d57bc7a6b89ef2 "$model" \
    | shasum -a 256 -c
  mise run rust:test:tall-scan-qualification
)
```

The task verifies the PDF against the pinned public test corpus. Page 5 and
Page 8 are isolated in a temporary directory from that verified source. Set
`XBERG_2502_PAGE5_ARCHIVE` or `XBERG_2502_PAGE8_ARCHIVE` to replay an existing
exact archive. The corresponding test verifies its archive digest.

Separate diagnostics reproduced the Guide Page 9 typo and both Nougat 009
misses in Xberg. The matched Nougat 010 checks passed. These cases are tracked
in [#2502](https://github.com/xberg-io/xberg/issues/2502).
