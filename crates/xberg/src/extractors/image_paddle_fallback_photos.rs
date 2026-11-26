//! Real reads of the benchmark photos, for the PaddleOCR image fallback tests.

pub(super) struct MeasuredPhoto {
    pub(super) name: &'static str,
    pub(super) tesseract: &'static [(&'static str, f64)],
    pub(super) paddle: &'static str,
}

/// Measured reads of the benchmark photos (2026-11-26): every Tesseract word with its
/// confidence, and PaddleOCR's text, from the default routes.
pub(super) const MEASURED_PHOTOS: &[MeasuredPhoto] = &[
    MeasuredPhoto {
        name: "cord_receipt_01",
        tesseract: &[
            (r#"2"#, 0.3220),
            (r#"&"#, 0.2759),
            (r#"ants"#, 0.0000),
            (r#"-TICKET"#, 0.8588),
            (r#"CP"#, 0.8592),
            (r#"7"#, 0.5372),
            (r#"60,000"#, 0.8423),
            (r#"£0,000"#, 0.8088),
            (r#"-"#, 0.0000),
            (r#"anes"#, 0.1796),
            (r#"tore."#, 0.2205),
            (r#"01s"#, 0.3953),
            (r#"$"#, 0.9130),
            (r#"~h0.000"#, 0.0955),
            (r#"TAX"#, 0.9118),
            (r#"5.455"#, 0.0903),
            (r#"Subtotal"#, 0.9057),
            (r#"40,000"#, 0.5866),
            (r#"TOTAL"#, 0.9670),
            (r#"caty"#, 0.7527),
            (r#"2.00"#, 0.0000),
            (r#"60,000"#, 0.0000),
            (r#"~"#, 0.3377),
            (r#"EDC"#, 0.9057),
            (r#"CINB"#, 0.4994),
            (r#"NIAGA"#, 0.7159),
            (r#"No:"#, 0.8348),
            (r#"xx7730"#, 0.8348),
            (r#"60.000"#, 0.7413),
            (r#"Seen"#, 0.0000),
            (r#"ae"#, 0.1088),
            (r#"sist"#, 0.1486),
            (r#"ieergen"#, 0.0000),
            (r#"TUC/,"#, 0.0000),
        ],
        paddle: r#"901016

-TICKET CP

2

60,000

60,000

TOTAL DISC S

-60.000

TAX

5.455

Subtotal

60.000

TOTAL (QtY 2.00 60.000

- EDC CIMB NIAGA No: Xx7730

60.000

CUSTONER THEO"#,
    },
    MeasuredPhoto {
        name: "cord_receipt_02",
        tesseract: &[
            (r#"vs"#, 0.3721),
            (r#"Ae."#, 0.3088),
            (r#"<,"#, 0.3750),
            (r#".-"#, 0.0000),
            (r#"a"#, 0.3082),
            (r#"sa)"#, 0.0000),
            (r#"pene"#, 0.3320),
            (r#"—"#, 0.6036),
            (r#"="#, 0.8880),
            (r#"rOTAL"#, 0.5175),
            (r#"391000"#, 0.7067),
            (r#"CA"#, 0.9555),
            (r#"391000"#, 0.7157),
        ],
        paddle: r#"J.STB PROMO

17500

Y.B.BAT

46000

Y.BASO PROM

27500

TOTAL

91000

91000

CASH"#,
    },
    MeasuredPhoto {
        name: "cord_receipt_03",
        tesseract: &[
            (r#"ISM"#, 0.2078),
            (r#"INE"#, 0.1951),
            (r#"MT"#, 0.1951),
            (r#"CoE"#, 0.1758),
            (r#"24,00"#, 0.9244),
            (r#"4,000"#, 0.8751),
            (r#"ICON]"#, 0.1376),
            (r#"ne?"#, 0.1918),
            (r#"Henle"#, 0.1452),
            (r#"<a"#, 0.2578),
            (r#"SUB"#, 0.7225),
            (r#"TU]"#, 0.4485),
            (r#"8,00"#, 0.5891),
            (r#"TOTAL"#, 0.9574),
            (r#"4"#, 0.7364),
            (r#")"#, 0.0000),
            (r#"28,"#, 0.5461),
            (r#"004"#, 0.7530),
            (r#"TOTAL"#, 0.7159),
            (r#"TMS"#, 0.4411),
            (r#"CASH"#, 0.8811),
            (r#"100,000"#, 0.6744),
            (r#"000"#, 0.8073),
            (r#"CHANGE"#, 0.8005),
            (r#"Www"#, 0.1395),
            (r#"Mos"#, 0.5785),
            (r#"112"#, 0.4393),
        ],
        paddle: r#"OSMINE MT(L)

24.000

UCONUT JELLY（L)

4.000

SUB TOTAL

28,000

TOTAL SALES

28.000

TOTAL JTEHS

CASH

100,000

72.000

CHANGE

Dueue No: 102"#,
    },
    MeasuredPhoto {
        name: "cord_receipt_04",
        tesseract: &[
            (r#"Qeheory"#, 0.2654),
            (r#"&"#, 0.8955),
            (r#"Cote"#, 0.6311),
            (r#"a"#, 0.5680),
            (r#"tt"#, 0.1649),
            (r#"vena"#, 0.0405),
            (r#"——"#, 0.3325),
            (r#"ee"#, 0.3347),
            (r#"<n"#, 0.1914),
            (r#"or"#, 0.1154),
            (r#"ae"#, 0.2389),
            (r#"a"#, 0.2721),
            (r#"a"#, 0.2766),
            (r#"@1100"#, 0.9028),
            (r#"ix"#, 0.7027),
            (r#"000"#, 0.9506),
            (r#"DONAT"#, 0.9165),
            (r#"GULA"#, 0.9250),
            (r#"1.00xITEMs"#, 0.1633),
            (r#"SUBTOTAL"#, 0.9686),
            (r#"1,000"#, 0.6576),
            (r#"TOTAL"#, 0.9552),
            (r#"11,000"#, 0.9117),
            (r#"CASH"#, 0.9661),
            (r#"11,000"#, 0.9137),
            (r#"oe"#, 0.3466),
            (r#"a"#, 0.3228),
            (r#"tf"#, 0.4770),
            (r#"a"#, 0.2854),
            (r#"in"#, 0.3444),
            (r#"wt"#, 0.2666),
            (r#"omen"#, 0.3464),
            (r#"ms"#, 0.4687),
            (r#"a"#, 0.3271),
            (r#"-"#, 0.6792),
        ],
        paddle: r#"@11000

1X

11,000

DONAT GULA

1.00xITEMs

11,000

SUBTOTAL

11,000

TOTAL

11,000

CASH"#,
    },
    MeasuredPhoto {
        name: "textocr_scene_01",
        tesseract: &[
            (r#"M"#, 0.8820),
            (r#"pees"#, 0.2547),
            (r#"VALUES"#, 0.9628),
            (r#"="#, 0.3984),
            (r#"<=="#, 0.0000),
            (r#"IN"#, 0.9656),
            (r#"OYD"#, 0.5659),
            (r#"————"#, 0.2493),
            (r#"HOPLEY"#, 0.0016),
            (r#"AND"#, 0.8611),
            (r#"wil"#, 0.6345),
            (r#"ONS"#, 0.1393),
            (r#"TOPLEL"#, 0.0000),
            (r#"xp)"#, 0.5223),
            (r#"WiLsi"#, 0.2810),
            (r#"Sa"#, 0.6264),
            (r#"meal"#, 0.1589),
            (r#"oR"#, 0.6473),
            (r#"SLINICAL"#, 0.6158),
            (r#"BERGEY's"#, 0.1784),
            (r#"MANUAL"#, 0.8753),
            (r#"PRIN"#, 0.9463),
            (r#"pre"#, 0.4932),
            (r#"“MEDICINE"#, 0.5417),
            (r#"ar."#, 0.4419),
            (r#"SUNDERMAN"#, 0.7589),
            (r#"BOERNER"#, 0.9204),
            (r#"IDIMION"#, 0.0000),
            (r#"|"#, 0.4945),
            (r#"¥"#, 0.0000),
            (r#"G."#, 0.5093),
            (r#"A"#, 0.6245),
            (r#"ot"#, 0.6067),
            (r#"5)"#, 0.0000),
            (r#"pate"#, 0.2280),
            (r#"ee"#, 0.1774),
            (r#"a"#, 0.7939),
            (r#"ae"#, 0.2623),
        ],
        paddle: r#"NORMAL

VALUES

BOYD

IN

TOPLEY AND

TOPLEL. AND

Herm

CLINICAL

WILSON'S

WILSON'S

BERGEY'S MANUAL

PRINCIPLES

PRINCIPLES

MEDICINE

OP

PREVENTIVE

OP

BACTERIOLOGY

OF

MEI

DETERMINATIVE

BACTERIOLOG

MEDICINE

AND

IMMUNITY

IMMUNITY

AND

BACTERIOLOGY

SUNDERMAN

THIRD EDITION

THIRD EDPHIO

BOERNER

REVISED BY

REVISED BY

SIXTH EDITION

7th EDITION

G.S.WILSON

O.S.WILSON

ENTOMOLOGY

AND

A.A.MILES

VOL.1"#,
    },
    MeasuredPhoto {
        name: "textocr_scene_03",
        tesseract: &[
            (r#"Ov,"#, 0.0000),
            (r#"LSS"#, 0.7721),
            (r#"7X"#, 0.1960),
            (r#"S"#, 0.1206),
            (r#"a)"#, 0.3337),
            (r#"ee"#, 0.0862),
            (r#"apie"#, 0.1465),
            (r#"as"#, 0.6888),
            (r#"a"#, 0.0612),
            (r#"CY"#, 0.1264),
            (r#"TER"#, 0.8423),
            (r#"re"#, 0.3165),
            (r#"ig"#, 0.3977),
            (r#"iu"#, 0.5487),
            (r#"Movie"#, 0.2222),
            (r#"a"#, 0.2295),
            (r#"a"#, 0.1359),
            (r#"~~"#, 0.5316),
            (r#"ay"#, 0.3669),
            (r#"Ten,"#, 0.2015),
            (r#"Wie"#, 0.6662),
            (r#"Sy,"#, 0.1396),
            (r#"~"#, 0.3606),
            (r#"EP"#, 0.3241),
            (r#"ms"#, 0.4140),
            (r#"}"#, 0.7259),
            (r#"aL"#, 0.5094),
            (r#"Ee,"#, 0.5632),
            (r#","#, 0.2360),
            (r#"2"#, 0.4236),
            (r#"S"#, 0.5954),
            (r#"ay"#, 0.3216),
            (r#"ia"#, 0.2240),
            (r#"a"#, 0.4169),
            (r#"_"#, 0.6367),
            (r#"ae"#, 0.3822),
            (r#"ee"#, 0.3341),
            (r#"——"#, 0.2268),
            (r#"="#, 0.3751),
            (r#"ad"#, 0.4951),
        ],
        paddle: r#"THE

LY IN THEATERS

MOVIE

TURYAS

TEENAGE MUTANT

TORY'ES

TEENAGE MUTANT

ONLY IN THEATERS

THE MOVIE

THE MOVIE

25000

ONLY IN THEATERS

COLLECT

N109525000

ALL 6

COLLECT

11

ALL 6

 2014 Par Pics.

2014 Par Plies"#,
    },
];
