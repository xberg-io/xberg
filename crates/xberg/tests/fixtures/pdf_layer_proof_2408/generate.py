"""Generate scan pages with a hidden number layer, using the Bitstream Vera font shipped by ReportLab.

Each page is one 150 DPI Poppler raster of a visible page, drawn over the whole page, plus an
invisible text layer. The variants separate what a raster proof must accept from what it must
refuse: a matching layer, a wrong digit at 11 pt, a flat dark band under 24 pt text, a blurred and
skewed scan, and a layer that is partly render mode 7.
"""

from pathlib import Path
from shutil import which
import subprocess
import sys
from tempfile import TemporaryDirectory

from PIL import Image, ImageFilter
from reportlab import rl_config
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfgen.canvas import Canvas
import reportlab

PAGE = (612, 792)
ORIGIN = (100, 650)
FONT_DIR = Path(reportlab.__file__).parent / "fonts"


def visible_scan(folder, size, prefix="Total ", digits="1234", suffix=" paid"):
    poppler = which("pdftoppm")
    if poppler is None:
        raise RuntimeError("pdftoppm is required")
    source = Path(folder) / "visible.pdf"
    canvas = Canvas(str(source), pagesize=PAGE, invariant=True)
    canvas.setFont("Vera", size)
    canvas.drawString(*ORIGIN, prefix + digits + suffix)
    canvas.save()
    subprocess.run([poppler, "-png", "-r", "150", "-singlefile", source, Path(folder) / "visible"], check=True)
    return Image.open(Path(folder) / "visible.png").convert("L")


def hidden(canvas, size, x, text, mode=3):
    layer = canvas.beginText(x, ORIGIN[1])
    layer.setFont("Vera", size)
    layer.setTextRenderMode(mode)
    layer.textOut(text)
    canvas.drawText(layer)


def page(path, image, draw_layer):
    with TemporaryDirectory() as folder:
        png = Path(folder) / "scan.png"
        image.save(png)
        canvas = Canvas(str(path), pagesize=PAGE, invariant=True)
        canvas.drawImage(str(png), 0, 0, *PAGE)
        draw_layer(canvas)
        canvas.save()


def generate(out):
    rl_config.useA85 = False
    pdfmetrics.registerFont(TTFont("Vera", str(FONT_DIR / "Vera.ttf")))
    out.mkdir(parents=True, exist_ok=True)
    with TemporaryDirectory() as folder:
        size = 11
        scan = visible_scan(folder, size)
        x = ORIGIN[0] + pdfmetrics.stringWidth("Total ", "Vera", size)
        page(out / "small-matching.pdf", scan, lambda c: hidden(c, 11, x, "1234"))
        page(out / "small-wrong.pdf", scan, lambda c: hidden(c, 11, x, "1284"))

        size = 24
        band = Image.new("L", (1275, 1650), 255)
        band.paste(20, (150, 220, 1100, 340))
        x = ORIGIN[0] + pdfmetrics.stringWidth("Total ", "Vera", size)
        page(out / "band.pdf", band, lambda c: hidden(c, size, x, "1234"))

        size = 24
        scan = visible_scan(folder, size)
        scan = scan.filter(ImageFilter.GaussianBlur(1.2)).rotate(0.6, resample=Image.BICUBIC, fillcolor=255)
        x = ORIGIN[0] + pdfmetrics.stringWidth("Total ", "Vera", size)
        page(out / "blurred-skewed.pdf", scan, lambda c: hidden(c, size, x, "1234"))

        scan = visible_scan(folder, size)
        half = pdfmetrics.stringWidth("12", "Vera", size)

        def partly_clipped(canvas):
            hidden(canvas, size, x, "12")
            hidden(canvas, size, x + half, "34", mode=7)

        page(out / "partly-mode7.pdf", scan, partly_clipped)

        scan = visible_scan(folder, size)
        page(out / "large-matching.pdf", scan, lambda c: hidden(c, size, x, "1234"))
        page(out / "large-wrong.pdf", scan, lambda c: hidden(c, size, x, "1284"))


if __name__ == "__main__":
    generate(Path(sys.argv[1]))
