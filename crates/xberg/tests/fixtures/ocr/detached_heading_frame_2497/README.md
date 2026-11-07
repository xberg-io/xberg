The one-page fixture is a 1276×1650 RGB raster at 150 ppi with no native
text layer. It uses Pillow 13.0.0 and DejaVu Sans at 20 px. The independently
generated cell oracle predates rendering and OCR. The random generator seed
is 3. Each row has two records, with labels selected by record ID modulo
eight from Cash, Rent, Travel, Repairs, Supplies, Taxes, Payroll, Sales.
Each record starts with its label and ID, followed by three independent
comma-separated integers drawn from 10000 through 99999 inclusive.

The heading begins at raster position (60, 90). The table starts at
y=210 with a 29 px row pitch. Columns begin at x=60, 230, 370, 510,
680, 850, 990 and 1130. The fixture PDF SHA-256 is
`0007e630efa28f59639975016a8a2560729af47a696f137920557d8bed16344f`.

The 90° control is the same raster rotated clockwise before PDF encoding,
with an unchanged cell oracle. Its SHA-256 is
`1128fc7e3586699c33b14372e71265e2cec284311c493a7a85ac1c0ac30073a0`.
