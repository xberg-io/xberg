"""Generate synthetic Word 97 fixtures for #2857 with LibreOffice (no external documents).

Run: python3 generate.py /Applications/LibreOffice.app/Contents/MacOS/soffice
The FODT source explicitly shares list identity across headings; restart and new-list
controls distinguish continuation from a global counter. Only the binary DOCs are retained.
"""

import pathlib
import subprocess
import sys
import tempfile

HEADER = '''<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
<office:styles><text:list-style style:name="Numbering">
<text:list-level-style-number text:level="1" style:num-format="1" style:num-suffix="."/>
</text:list-style><text:list-style style:name="Independent">
<text:list-level-style-number text:level="1" style:num-format="1" style:num-suffix="."/>
</text:list-style><text:list-style style:name="Seven">
<text:list-level-style-number text:level="1" style:num-format="1" style:num-suffix="." text:start-value="7"/>
</text:list-style></office:styles><office:body><office:text>'''


def heading(text: str) -> str:
    return f'<text:h text:outline-level="1">{text}</text:h>'


def items(identity: str, labels: list[str], continuation: str = "", style: str = "Numbering") -> str:
    attrs = f' text:continue-list="{continuation}"' if continuation else ""
    result = f'<text:list xml:id="{identity}" text:style-name="{style}"{attrs}>'
    for label in labels:
        result += f'<text:list-item><text:p>{label}</text:p></text:list-item>'
    return result + "</text:list>"


def sources() -> dict[str, str]:
    first = items("first", ["First paragraph.", "Second paragraph."])
    second = ["Third paragraph.", "Fourth paragraph."]
    return {
        "continuous": heading("Background") + first + heading("Discussion")
        + items("second", second, "first") + heading("Conclusion")
        + items("third", ["Fifth paragraph.", "Sixth paragraph."], "second"),
        "restart": heading("Before restart") + first + heading("After restart")
        + items("second", second, style="Independent"),
        "new_list": heading("First list") + first + heading("Independent list")
        + items("second", second, style="Independent"),
        "start_seven": heading("Nondefault start")
        + items("first", ["Seventh paragraph.", "Eighth paragraph."], style="Seven")
        + heading("Continued") + items("second", ["Ninth paragraph."], "first", "Seven"),
    }


def main() -> None:
    destination = pathlib.Path(__file__).resolve().parent
    with tempfile.TemporaryDirectory(prefix="doc2857-") as directory:
        scratch = pathlib.Path(directory)
        for name, body in sources().items():
            source = scratch / f"{name}.fodt"
            source.write_text(HEADER + body + "</office:text></office:body></office:document>")
            subprocess.run(
                [sys.argv[1], f"-env:UserInstallation={(scratch / 'profile').as_uri()}",
                 "--headless", "--convert-to", "doc:MS Word 97", "--outdir", str(destination), str(source)],
                check=True,
            )
            data = (destination / f"{name}.doc").read_bytes()
            if not data.startswith(bytes.fromhex("d0cf11e0a1b11ae1")):
                raise ValueError("LibreOffice did not produce a binary OLE DOC")


if __name__ == "__main__":
    main()
