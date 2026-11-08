Synthetic one-page PDFs for `pdf_picture_prose_auto_2327.rs`: prose rendered into a raster
picture that covers part of a Letter page, beside a heading-sized text layer.

Regenerate (needs pillow and reportlab, and a serif TrueType font):

    python3 -I g2327.py /tmp/out

then copy the six PDFs used by the test (`half_short`, `half_long`, `side_long`, `two_short`,
`photo_long`, `photo_short`) into `pdfs/`. The generator writes more placements than the test uses.
