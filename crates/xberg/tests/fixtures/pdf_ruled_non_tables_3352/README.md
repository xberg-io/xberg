One gzip-compressed one-page PDF with invented text and painted vector rules.
It holds two regions that must stay out of the table output: three-column
justified prose inside a painted box (its top and bottom rules span the text, so
the ruled admission branch applies) and a block of short axis ticks with numeric
labels (its rules are shorter than the text, so they are not the region's own).

`python3 generate.py` rewrites the file byte for byte.
