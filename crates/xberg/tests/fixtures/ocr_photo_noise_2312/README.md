# Clean-text controls for the photo symbol-noise filter (#2312)

`prose.png` is six lines of black Arial 44 on a white 1700 x 1100 canvas. `prose.pdf` is the same
six lines as a native Helvetica 12 text layer. Neither contains a photograph, so the symbol-noise
filter must leave their text unchanged.

`prose-then-photo.pdf` has a native-text page followed by a page holding only a 1275 x 1650
JPEG (quality 60) of random noise at four scales, drawn across the page. It pins the mixed route.
