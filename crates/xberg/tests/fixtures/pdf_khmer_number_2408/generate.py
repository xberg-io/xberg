"""Generate a matched scan pair using the pinned Noto Sans Khmer font."""

from hashlib import sha256
from pathlib import Path
from shutil import which
import subprocess
import sys
from tempfile import TemporaryDirectory

from reportlab import rl_config
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfgen.canvas import Canvas

FONT_SHA256 = "e66675f2082788f0511a714bef5a1748928294b38c8e286a96ea73a864b5e605"
PAGE = (612, 792)
DIGITS = {"matching": "ប០ប", "misleading": "ប១ប"}
PROSE = (
    "The office keeps the current monthly service records.",
    "Readers can compare each line with its source record.",
    "The committee will meet on Monday to discuss the schedule.",
    "Please retain the signed copy for your own records.",
    "The annual report is available at the information desk.",
    "The public document includes the relevant details.",
)


def native_word(canvas, word, invisible):
    text = canvas.beginText(100, 650)
    text.setFont("Khmer", 30)
    text.setTextRenderMode(3 if invisible else 0)
    text.textOut(word)
    canvas.drawText(text)


def generate_lookalikes(output, poppler):
    latin = Path(__file__).resolve().parents[4] / "xberg-native-pdf/src/fonts/assets/DejaVuSans.ttf"
    assert sha256(latin.read_bytes()).hexdigest() == "abdc775b21b1bc470d50c97e790d276f2054b7504e56e5bd3e64f48d68582322"
    pdfmetrics.registerFont(TTFont("Latin", str(latin)))
    flank = pdfmetrics.stringWidth("ប", "Khmer", 30)
    digit = pdfmetrics.stringWidth("០", "Khmer", 30)
    for name, glyph in [("painted_latin_o", "O"), ("painted_ascii_zero", "0")]:
        with TemporaryDirectory() as folder:
            visible = Path(folder) / "visible"
            canvas = Canvas(str(visible.with_suffix(".pdf")), pagesize=PAGE, invariant=True)
            canvas.setFont("Khmer", 30)
            canvas.drawString(100, 650, "ប")
            canvas.drawString(100 + flank + digit, 650, "ប")
            canvas.saveState()
            canvas.translate(100 + flank, 650)
            canvas.scale(digit / pdfmetrics.stringWidth(glyph, "Latin", 30), 1)
            canvas.setFont("Latin", 30)
            canvas.drawString(0, 0, glyph)
            canvas.restoreState()
            canvas.setFont("Helvetica", 15)
            for row, text in enumerate(PROSE):
                canvas.drawString(100, 600 - 30 * row, text)
            canvas.save()
            subprocess.run(
                [poppler, "-png", "-r", "150", "-singlefile", visible.with_suffix(".pdf"), visible], check=True
            )
            canvas = Canvas(str(output / f"{name}.pdf"), pagesize=PAGE, invariant=True)
            canvas.drawImage(str(visible.with_suffix(".png")), 0, 0, *PAGE)
            native_word(canvas, DIGITS["matching"], invisible=True)
            canvas.save()


def generate(font, output, lookalikes_only=False):
    assert sha256(font.read_bytes()).hexdigest() == FONT_SHA256
    poppler = which("pdftoppm")
    if poppler is None:
        raise RuntimeError("pdftoppm is required to render the source scan")
    rl_config.useA85 = False
    pdfmetrics.registerFont(TTFont("Khmer", str(font)))
    output.mkdir(parents=True, exist_ok=True)
    if lookalikes_only:
        generate_lookalikes(output, poppler)
        return
    with TemporaryDirectory() as folder:
        visible = Path(folder) / "visible"
        canvas = Canvas(str(visible.with_suffix(".pdf")), pagesize=PAGE, invariant=True)
        native_word(canvas, DIGITS["matching"], invisible=False)
        canvas.setFont("Helvetica", 15)
        for row, text in enumerate(PROSE):
            canvas.drawString(100, 600 - 30 * row, text)
        canvas.save()
        subprocess.run([poppler, "-png", "-r", "150", "-singlefile", visible.with_suffix(".pdf"), visible], check=True)
        for kind, word in DIGITS.items():
            canvas = Canvas(str(output / f"{kind}.pdf"), pagesize=PAGE, invariant=True)
            canvas.drawImage(str(visible.with_suffix(".png")), 0, 0, *PAGE)
            native_word(canvas, word, invisible=True)
            canvas.save()


if __name__ == "__main__":
    generate(Path(sys.argv[1]), Path(sys.argv[2]), "--lookalikes-only" in sys.argv[3:])
