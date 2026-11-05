use lopdf::{Document, Object, Stream, dictionary};

fn source_width(text: &str) -> f64 {
    text.chars()
        .map(|character| match character {
            'i' | 'j' | 'l' => 0.22,
            ' ' | 'f' | 't' => 0.28,
            _ => 0.56,
        })
        .sum::<f64>()
        * 8.0
}

fn panel_stream(tilt_degrees: f64, panels: usize, width: fn(&str) -> f64, precision: usize) -> (String, Vec<String>) {
    let words: Vec<_> = "harbor meadow lantern orchard compass thimble granite willow ferry quarry pasture beacon \
        cobble saddle marble timber anchor bridge cellar dune embers fjord glacier hollow island juniper kettle \
        lagoon mast nectar oasis pebble"
        .split_whitespace()
        .collect();
    let cuts = [14, 2, 9, 1, 12, 3, 10, 2];
    let mut operators = Vec::new();
    let mut rows = Vec::new();
    for x in [72.0, 320.0].into_iter().take(panels) {
        for row in 0..4 {
            let text = (0..6)
                .map(|word| words[(rows.len() * 5 + word) % words.len()])
                .collect::<Vec<_>>()
                .join(" ");
            let (mut start, mut cut) = (0, 0);
            while start < text.len() {
                let end = (start + cuts[cut % cuts.len()]).min(text.len());
                let offset = width(&text[..start]);
                let y = 700.0 - 9.0 * f64::from(row) + offset * tilt_degrees.to_radians().tan();
                operators.push(format!(
                    "BT /F1 8 Tf 1 0 0 1 {:.precision$} {y:.precision$} Tm ({}) Tj ET",
                    x + offset,
                    &text[start..end]
                ));
                start = end;
                cut += 1;
            }
            rows.push(text);
        }
    }
    (operators.join("\n"), rows)
}

pub fn issue_2323_panels(tilt_degrees: f64, panels: usize) -> (Vec<u8>, Vec<String>) {
    build_panels(tilt_degrees, panels, source_width, 2)
}

pub fn accurate_helvetica_panels(tilt_degrees: f64) -> (Vec<u8>, Vec<String>) {
    build_panels(tilt_degrees, 2, helvetica_width, 6)
}

// Adobe Helvetica AFM 002.000 advances, frozen independently of extractor metrics. ~keep
const HELVETICA_LOWERCASE_ADVANCES: [u16; 26] = [
    556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722,
    500, 500, 500,
];

fn helvetica_width(text: &str) -> f64 {
    let units: u32 = text
        .bytes()
        .map(|byte| match byte {
            b' ' => 278,
            b'a'..=b'z' => u32::from(HELVETICA_LOWERCASE_ADVANCES[usize::from(byte - b'a')]),
            _ => panic!("AFM fixture contains an unsupported character"),
        })
        .sum();
    f64::from(units) * 8.0 / 1000.0
}

fn build_panels(tilt_degrees: f64, panels: usize, width: fn(&str) -> f64, precision: usize) -> (Vec<u8>, Vec<String>) {
    assert!((1..=2).contains(&panels));
    let (stream, rows) = panel_stream(tilt_degrees, panels, width, precision);
    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" });
    let content = pdf.add_object(Stream::new(dictionary! {}, stream.into_bytes()));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => content,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
        }),
    );
    let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes)
        .expect("serialize original issue panel operators");
    (bytes, rows)
}
