use super::{line, paragraph};
use crate::pdf::structure::types::PdfParagraph;

pub(super) fn hyphen_candidate() -> PdfParagraph {
    paragraph(
        vec![line(
            317.759888,
            9.962640,
            true,
            &[
                ("feedback", 311.999329, 35.383568, 9.962640, true, false),
                ("controller", 353.039520, 39.515343, 9.962640, true, false),
                ("is", 398.399292, 6.635487, 9.962640, true, false),
                ("able", 410.999298, 17.263432, 9.962640, true, false),
                ("to", 434.039185, 7.741315, 9.962640, true, false),
                ("achieve", 447.719299, 30.343578, 9.962640, true, false),
                ("the", 483.719421, 12.223419, 9.962640, true, false),
                ("user", 501.839386, 17.195511, 9.962640, true, false),
                ("specified-", 524.879272, 38.400082, 9.962640, true, false),
            ],
        )],
        9.962640,
        true,
    )
}

pub(super) fn hyphen_body() -> PdfParagraph {
    paragraph(
        vec![
            line(
                305.759888,
                8.281446,
                true,
                &[
                    ("performance", 311.999329, 51.103428, 9.962640, true, false),
                    ("in", 366.719238, 7.741340, 9.962640, true, false),
                    ("terms", 378.359100, 22.115479, 9.962640, true, false),
                    ("of", 404.398987, 7.809602, 9.962640, true, false),
                    ("the", 416.038818, 12.223419, 9.962640, true, false),
                    ("closed-loop", 431.998993, 46.741394, 9.962640, true, false),
                    ("systems", 482.399078, 30.359991, 9.962640, true, false),
                    ("H", 516.719360, 8.400022, 9.962640, true, false),
                    ("2", 525.119751, 3.960015, 6.973850, true, false),
                    ("or", 533.399841, 8.879982, 9.962640, true, false),
                    ("H", 546.239624, 8.400022, 9.962640, true, false),
                    ("∞", 554.639648, 7.920030, 6.973850, true, false),
                ],
            ),
            line(
                293.759857,
                9.962640,
                true,
                &[("norms.", 311.999634, 27.480024, 9.962640, true, false)],
            ),
        ],
        9.962640,
        true,
    )
}

pub(super) fn foreign_bold_body() -> PdfParagraph {
    paragraph(
        vec![
            line(
                309.119995,
                9.962640,
                true,
                &[
                    ("control", 48.960262, 28.449471, 9.962640, true, false),
                    ("magnitude", 81.000107, 42.223320, 9.962640, true, false),
                    ("and", 126.600113, 15.061320, 9.962640, true, false),
                    ("rate,", 145.200119, 18.330503, 9.962640, true, false),
                    ("is", 167.159897, 6.635462, 9.962640, true, false),
                    ("given", 177.479828, 21.661304, 9.962640, true, false),
                    ("by", 202.679825, 9.463400, 9.962640, true, false),
                    ("the", 215.759918, 12.223419, 9.962640, true, false),
                    ("following", 231.599777, 37.741325, 9.962640, true, false),
                    ("convex", 272.999817, 27.223404, 9.962640, true, false),
                ],
            ),
            line(
                297.119995,
                9.962640,
                true,
                &[
                    ("optimization", 48.960262, 49.981323, 9.962640, true, false),
                    ("problem,", 102.240341, 35.399990, 9.962640, true, false),
                ],
            ),
        ],
        9.962640,
        true,
    )
}

pub(super) fn two_cell_header() -> PdfParagraph {
    paragraph(
        vec![line(
            683.532959,
            10.000000,
            true,
            &[
                ("Sample Size", 114.195999, 58.300003, 10.000000, true, false),
                ("Test Statistic", 185.805405, 61.456207, 10.000000, true, false),
            ],
        )],
        10.000000,
        true,
    )
}

pub(super) fn formula_rows() -> PdfParagraph {
    let mut rows = unknown_sigma_rows();
    rows.extend(known_sigma_rows());
    paragraph(rows, 8.0, false)
}

fn unknown_sigma_rows() -> Vec<crate::pdf::structure::types::PdfLine> {
    vec![
        line(
            670.875000,
            8.000000,
            false,
            &[("–", 218.988007, 4.000000, 8.000000, false, false)],
        ),
        line(
            663.123962,
            8.968750,
            false,
            &[
                ("< 30", 137.748993, 18.734001, 9.500000, false, false),
                ("X", 217.858002, 6.110001, 10.000000, false, true),
                ("-", 227.433014, 3.330000, 10.000000, false, false),
                ("µ", 233.305008, 5.000000, 10.000000, false, true),
                ("0", 239.222015, 4.000000, 8.000000, false, false),
            ],
        ),
        line(
            656.982971,
            9.500000,
            false,
            &[
                ("t", 196.722000, 2.780000, 10.000000, false, true),
                ("c", 200.158997, 3.552000, 8.000000, false, true),
                ("=", 206.481003, 6.850000, 10.000000, false, false),
            ],
        ),
        line(
            651.723938,
            9.937500,
            false,
            &[
                ("(σ unknown)", 120.696991, 52.839008, 9.500000, false, false),
                ("s", 218.246017, 3.890000, 10.000000, false, true),
                ("/", 223.873016, 2.780000, 10.000000, false, false),
                ("n", 236.743011, 5.000000, 10.000000, false, true),
            ],
        ),
    ]
}

fn known_sigma_rows() -> Vec<crate::pdf::structure::types::PdfLine> {
    vec![
        line(
            637.060059,
            8.000000,
            false,
            &[("–", 220.403000, 4.000000, 8.000000, false, false)],
        ),
        line(
            629.309021,
            8.968750,
            false,
            &[
                ("< 30", 137.748001, 18.734001, 9.500000, false, false),
                ("X", 219.272995, 6.110001, 10.000000, false, true),
                ("-", 228.847992, 3.330000, 10.000000, false, false),
                ("µ", 234.719986, 5.000000, 10.000000, false, true),
                ("0", 240.636993, 4.000000, 8.000000, false, false),
            ],
        ),
        line(
            623.168030,
            9.500000,
            false,
            &[
                ("Z", 195.356995, 5.560000, 10.000000, false, true),
                ("c", 201.573990, 3.552000, 8.000000, false, true),
                ("=", 207.895996, 6.850000, 10.000000, false, false),
            ],
        ),
        line(
            617.908997,
            9.937500,
            false,
            &[
                ("(σ known)", 125.980003, 42.275009, 9.500000, false, false),
                ("σ", 218.950989, 4.980000, 10.000000, false, true),
                ("/", 225.997986, 2.780000, 10.000000, false, false),
                ("n", 238.867981, 5.000000, 10.000000, false, true),
            ],
        ),
    ]
}

pub(super) fn existing_section() -> PdfParagraph {
    paragraph(
        vec![line(
            715.432983,
            10.000000,
            true,
            &[
                ("9.3", 72.000000, 13.890001, 10.000000, true, false),
                (
                    "Distribution Needed for Hypothesis Testing",
                    88.679703,
                    205.357498,
                    10.000000,
                    true,
                    false,
                ),
            ],
        )],
        10.000000,
        true,
    )
}
