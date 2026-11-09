Synthetic one-page PDFs for `pdf_picture_prose_auto_2327.rs`: prose rendered into a raster
picture that covers part of a Letter page, beside a heading-sized text layer.

Regenerate (needs pillow, reportlab and a serif TrueType font) from this directory:

    python3 -I g2327.py pdfs

The output is byte-identical between runs (`invariant=1`), so a rerun leaves no diff.
