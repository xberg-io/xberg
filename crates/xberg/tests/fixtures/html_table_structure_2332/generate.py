"""Write the deterministic GH#2332 HTML table fixtures next to this script.

Each fixture isolates one structural property the rendered HTML must keep:

- ``spans-and-two-tier-header.html``: a two-tier header whose upper tier both
  spans a column (``colspan=2``) and reaches into the leaf tier (``rowspan=2``),
  over a body whose first cell spans two rows.
- ``short-row.html``: a row with fewer cells than the grid, which is padding,
  not a merge.
- ``headerless-body-rows.html``: a table with no header cell anywhere, so its
  first row is body data.
- ``blank-gutter-tables.html``: two sibling tables separated only by a blank
  gutter (whitespace), with no prose between them.
"""

from pathlib import Path

FIXTURES = {
    "spans-and-two-tier-header.html": """<table>
<thead>
<tr><th rowspan="2">Item</th><th colspan="2">Measurement</th></tr>
<tr><th>Value</th><th>Unit</th></tr>
</thead>
<tbody>
<tr><td rowspan="2">Widget</td><td>10</td><td>mm</td></tr>
<tr><td>20</td><td>mm</td></tr>
</tbody>
</table>
""",
    "short-row.html": """<table>
<tr><th>A</th><th>B</th><th>C</th></tr>
<tr><td>1</td></tr>
<tr><td>2</td><td>3</td><td>4</td></tr>
</table>
""",
    "headerless-body-rows.html": """<table>
<tr><td>Alice</td><td>10</td></tr>
<tr><td>Bob</td><td>20</td></tr>
</table>
""",
    "blank-gutter-tables.html": """<table>
<tr><th>Zone</th><th>Capacity</th></tr>
<tr><td>North</td><td>350</td></tr>
</table>

<table>
<tr><th>Zone</th><th>Capacity</th></tr>
<tr><td>East</td><td>470</td></tr>
</table>
""",
}


def main() -> None:
    folder = Path(__file__).resolve().parent
    for name, content in FIXTURES.items():
        (folder / name).write_text(content, encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
