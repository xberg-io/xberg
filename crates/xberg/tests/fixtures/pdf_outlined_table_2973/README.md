# Synthetic outlined table for issue #2973

The fixture uses invented data from the [issue reproducer](https://github.com/xberg-io/xberg/issues/2973).
The generator uses the Bitstream Vera font bundled with ReportLab. Headings use PDF text operators
and the body uses filled glyph contours. The generator enables invariant PDF output
for reproducible timestamps and IDs.

From this directory, regenerate with:

```sh
uv run --no-project --with reportlab==4.4.4 --with fonttools==4.60.1 python vector_table.py table table.pdf
uv run --no-project --with reportlab==4.4.4 --with fonttools==4.60.1 python vector_table.py table short-heading.pdf 10 short
```

SHA-256 digests:

- `table.pdf` `4b5235625968b73b65ad757ceb8e72cd63256b7ec8ec845908d6b7f6ecb80b58`
- `short-heading.pdf` `c89305f914d79458fcd2f6c43d862f1edde033ca02901370d888650479815175`

Additional controls use the same generator and dependency pins:

```sh
uv run --no-project --with reportlab==4.4.4 --with fonttools==4.60.1 python vector_table.py geometry geometry.pdf 10
uv run --no-project --with reportlab==4.4.4 --with fonttools==4.60.1 python vector_table.py prose prose.pdf 12
uv run --no-project --with reportlab==4.4.4 --with fonttools==4.60.1 python vector_table.py headless headless.pdf 10
```

- `geometry.pdf` `3478dccb6b9b7a6349746b1b659d4c81285dc3bcc1692f97dbafef1ae6d3ba21`
- `prose.pdf` `17e892a4db189dd5d5588a78eb11a98aa5d9fe4ec284a815af28587f3cc67d39`
- `headless.pdf` `390bef0cfcf85ec8ca1eb5551b17aca78fc18724e66d1ca1c0b50f68460e5cbd`

The geometry control has seven filled bars below the native heading. The prose
control uses the reporter's positive 12 pt outlined-prose variant. The headless
control uses the same table paths with no native heading.
