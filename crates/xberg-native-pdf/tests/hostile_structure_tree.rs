//! Structure-tree parsing must survive a cyclic page tree and arbitrarily deep
//! element chains, and resolve /Pg through ancestors (ISO 32000-1 §14.7.4:
//! a missing /Pg is inherited, not page 0). Each walk runs on a small stack so
//! unbounded recursion aborts the test binary instead of passing unnoticed.

mod common;

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::structure::{StructChild, StructElem, StructTreeRoot, traverse_structure_tree_all_pages};

fn tree_depth(elem: &StructElem) -> usize {
    1 + elem
        .children
        .iter()
        .filter_map(|c| match c {
            StructChild::StructElem(e) => Some(tree_depth(e)),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

fn mcr_pages(elem: &StructElem, out: &mut Vec<(u32, u32)>) {
    for child in &elem.children {
        match child {
            StructChild::MarkedContentRef { mcid, page, .. } => out.push((*mcid, *page)),
            StructChild::StructElem(e) => mcr_pages(e, out),
            _ => {}
        }
    }
}

fn parse(pdf: Vec<u8>) -> Option<StructTreeRoot> {
    common::on_small_stack(move || {
        let doc = PdfDocument::from_bytes(pdf).expect("parse");
        doc.structure_tree().expect("structure tree")
    })
}

#[test]
fn cyclic_page_tree_with_struct_tree_root_terminates() {
    let pdf = common::build_pdf_from_objects(&[
        (1, "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R >>".into()),
        (2, "<< /Type /Pages /Kids [2 0 R 2 0 R 3 0 R] /Count 7 >>".into()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>".into()),
        (6, "<< /Type /StructTreeRoot /K 7 0 R >>".into()),
        (7, "<< /Type /StructElem /S /Document /Pg 3 0 R /K 0 >>".into()),
    ]);
    let tree = parse(pdf).expect("tagged");
    assert_eq!(tree.root_elements.len(), 1);
}

#[test]
fn long_acyclic_element_chain_is_truncated() {
    const LEN: u32 = 50_000;
    let mut objects: Vec<(u32, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R >>".into()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>".into()),
        (6, "<< /Type /StructTreeRoot /K 7 0 R >>".into()),
    ];
    for id in 7..7 + LEN {
        let kid = if id + 1 < 7 + LEN {
            format!("{} 0 R", id + 1)
        } else {
            "0".into()
        };
        objects.push((id, format!("<< /Type /StructElem /S /Sect /K {kid} >>")));
    }
    let pdf = common::build_pdf_from_objects(&objects);
    let depth = common::on_small_stack(move || {
        let doc = PdfDocument::from_bytes(pdf).expect("parse");
        let tree = doc.structure_tree().expect("structure tree").expect("tagged");
        let depth = tree_depth(&tree.root_elements[0]);
        traverse_structure_tree_all_pages(&tree);
        depth
    });
    assert!(depth <= 130, "depth cap must truncate the chain, got {depth}");
    assert!(depth > 50, "the cap must not be tiny, got {depth}");
}

fn pg_inheritance_pdf(page_type: &str, pages_type: &str) -> Vec<u8> {
    common::build_pdf_from_objects(&[
        (1, "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R >>".into()),
        (2, format!("<< {pages_type} /Kids [3 0 R 4 0 R] /Count 2 >>")),
        (3, format!("<< {page_type} /Parent 2 0 R /MediaBox [0 0 200 200] >>")),
        (4, format!("<< {page_type} /Parent 2 0 R /MediaBox [0 0 200 200] >>")),
        (6, "<< /Type /StructTreeRoot /K 7 0 R >>".into()),
        (7, "<< /Type /StructElem /S /Sect /Pg 4 0 R /K [8 0 R] >>".into()),
        (
            8,
            "<< /Type /StructElem /S /P /K [<< /Type /MCR /MCID 3 >> 5] >>".into(),
        ),
    ])
}

#[test]
fn leaves_without_pg_resolve_to_nearest_ancestor_page() {
    let tree = parse(pg_inheritance_pdf("/Type /Page", "/Type /Pages")).expect("tagged");
    let mut found = Vec::new();
    mcr_pages(&tree.root_elements[0], &mut found);
    found.sort();
    assert_eq!(found, vec![(3, 1), (5, 1)]);
}

#[test]
fn page_tree_without_type_entries_still_maps_pg() {
    let tree = parse(pg_inheritance_pdf("", "")).expect("tagged");
    let mut found = Vec::new();
    mcr_pages(&tree.root_elements[0], &mut found);
    found.sort();
    assert_eq!(found, vec![(3, 1), (5, 1)]);
}
