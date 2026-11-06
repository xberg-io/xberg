# DejaVu ligature source-space regression

The PDF uses the issue's ReportLab recipe with DejaVu Serif at 14 points and
one additional phrase to verify the source space after the final ligature in
`staﬀ only`. The PDF contains one text-show operator, explicit byte `20`
between those words, and a ToUnicode CMap mapping `01` to `FB00`, `02` to
`FB01`, `03` to `FB02`, and `04` to `FB04`.

The embedded subset comes from
[DejaVu Serif 2.37](https://github.com/dejavu-fonts/dejavu-fonts/releases/tag/version_2_37).
The upstream TTF SHA-256 is
`42d1edeb7952f31b1f96d767ed7030b08a39e0c372b0071641518864e2bffb51`.
The accompanying file contains the upstream license.

To reproduce with ReportLab 4.4.9, create a letter-sized canvas with
`invariant=True`, register the TTF as `Serif`, set that font at 14 points,
and draw `diﬀerent deﬁnes ﬂow; staﬀ only, waﬄe ﬂoor` at `(72, 700)`.
