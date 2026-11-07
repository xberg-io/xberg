use super::*;

fn outline(x0: f32, y0: f32, x1: f32, y1: f32) -> Outline {
    Outline {
        bbox: Rect { x0, y0, x1, y1 },
        shape: DiagramShape::Box,
        fill: None,
        stroke: None,
        stroke_width: None,
        dashed: false,
    }
}

fn connector(start: (f32, f32), end: (f32, f32)) -> Connector {
    Connector {
        start,
        end,
        midpoint: ((start.0 + end.0) / 2.0, (start.1 + end.1) / 2.0),
        stroke: None,
        dashed: false,
    }
}

fn styled_connector(start: (f32, f32), end: (f32, f32), stroke: &str) -> Connector {
    Connector {
        stroke: Some(stroke.to_string()),
        ..connector(start, end)
    }
}

fn label(x: f32, y: f32, text: &str) -> Label {
    Label {
        x,
        y,
        text: text.to_string(),
    }
}

/// Landing points for a drawing whose connectors reach none of its shapes.
fn nowhere(outlines: usize) -> Vec<Vec<(f32, f32)>> {
    vec![Vec::new(); outlines]
}

/// A box drawn behind an edge label so the line does not run through the
/// text is the label's background, not a node. Mermaid states it as a CSS
/// fill on an HTML label, which no SVG path carries and a PDF writer turns
/// into a real filled rectangle, so only the PDF side ever sees it.
#[test]
fn a_box_behind_an_edge_label_is_not_a_node() {
    let mut background = outline(55.0, 115.0, 75.0, 135.0);
    background.fill = Some("#e8e8e8".to_string());

    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![
            outline(0.0, 0.0, 100.0, 50.0),
            background,
            outline(0.0, 200.0, 100.0, 250.0),
        ],
        vec![connector((50.0, 50.0), (50.0, 200.0))],
        vec![
            label(50.0, 25.0, "Start"),
            label(65.0, 125.0, "yes"),
            label(50.0, 225.0, "End"),
        ],
    )
    .expect("graph");

    let names: Vec<&str> = graph.nodes.iter().map(|n| n.label.as_str()).collect();
    assert_eq!(names, ["Start", "End"]);
    // The text the background held returns to the edge it was written on.
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].label.as_deref(), Some("yes"));
}

/// Only text sitting on a connector reads as an edge label. A small node
/// whose caption is nowhere near a connector keeps its own name.
#[test]
fn a_small_node_away_from_every_connector_keeps_its_label() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![
            outline(0.0, 0.0, 100.0, 50.0),
            outline(300.0, 300.0, 320.0, 320.0),
            outline(0.0, 200.0, 100.0, 250.0),
        ],
        vec![connector((50.0, 50.0), (50.0, 200.0))],
        vec![
            label(50.0, 25.0, "Start"),
            label(50.0, 225.0, "End"),
            label(310.0, 310.0, "Aside"),
        ],
    )
    .expect("graph");

    let names: Vec<&str> = graph.nodes.iter().map(|n| n.label.as_str()).collect();
    assert_eq!(names, ["Start", "End", "Aside"]);
}

/// A connector leaving a node and returning to it is a self-loop, and the
/// arc has to bulge clear of the node or nobody could see it.
#[test]
fn a_self_loop_arcing_clear_of_its_node_is_an_edge() {
    let mut loop_back = connector((30.0, 200.0), (70.0, 200.0));
    loop_back.midpoint = (50.0, 280.0);

    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![outline(0.0, 0.0, 100.0, 50.0), outline(0.0, 200.0, 100.0, 250.0)],
        vec![connector((50.0, 50.0), (50.0, 200.0)), loop_back],
        Vec::new(),
    )
    .expect("graph");

    let self_loop = graph.edges.iter().find(|e| e.from == e.to);
    assert!(self_loop.is_some(), "a self-loop is an edge: {:?}", graph.edges);
    assert_eq!(self_loop.expect("checked").from, 1);
}

/// A ruled table on the same page as a diagram is one closed rectangle with
/// text in it, which is a node in every respect except that its text is
/// laid out in rows *and* columns.
#[test]
fn a_box_holding_a_grid_of_text_is_a_table_not_a_node() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![
            outline(0.0, 0.0, 100.0, 50.0),
            outline(0.0, 200.0, 100.0, 250.0),
            outline(200.0, 0.0, 380.0, 100.0),
        ],
        vec![connector((50.0, 50.0), (50.0, 200.0))],
        vec![
            label(240.0, 20.0, "Stage"),
            label(320.0, 20.0, "Owner"),
            label(240.0, 70.0, "Build"),
            label(320.0, 70.0, "Ada"),
        ],
    )
    .expect("graph");

    assert_eq!(graph.nodes.len(), 2, "the table is not a node: {:?}", graph.nodes);
}

/// A caption wrapped onto several lines is many rows in one column, which
/// is not a grid however many lines it runs to.
#[test]
fn a_wrapped_caption_is_not_a_grid() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![outline(0.0, 0.0, 100.0, 50.0), outline(0.0, 200.0, 100.0, 250.0)],
        vec![connector((50.0, 50.0), (50.0, 200.0))],
        vec![
            label(50.0, 210.0, "Release"),
            label(50.0, 225.0, "engineer"),
            label(50.0, 240.0, "on call"),
        ],
    )
    .expect("graph");

    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.nodes[1].label, "Release\nengineer\non call");
}

/// Pie wedges all meet at the centre, so their boxes overlap. Nothing a
/// layout engine draws does that, and none of them is named.
#[test]
fn overlapping_anonymous_shapes_are_one_drawing_not_nodes() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![
            outline(0.0, 0.0, 100.0, 50.0),
            outline(0.0, 200.0, 100.0, 250.0),
            outline(200.0, 100.0, 300.0, 200.0),
            outline(210.0, 110.0, 310.0, 210.0),
        ],
        vec![connector((50.0, 50.0), (50.0, 200.0))],
        vec![label(50.0, 25.0, "from"), label(50.0, 225.0, "to")],
    )
    .expect("graph");

    assert_eq!(
        graph.nodes.len(),
        2,
        "overlapping wedges are not nodes: {:?}",
        graph.nodes
    );
}

/// Overlap only condemns a shape nobody named. Two labelled nodes placed
/// close enough to overlap are still two nodes.
#[test]
fn overlapping_labelled_shapes_are_still_nodes() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![outline(0.0, 0.0, 100.0, 100.0), outline(60.0, 60.0, 160.0, 160.0)],
        vec![connector((50.0, 50.0), (110.0, 110.0))],
        vec![label(20.0, 20.0, "left"), label(140.0, 140.0, "right")],
    )
    .expect("graph");

    assert_eq!(graph.nodes.len(), 2);
}

/// A terminal dot beside labelled activities is decoration, but a shape the
/// size of its labelled neighbours is a node even unnamed.
#[test]
fn a_tiny_unlabelled_shape_beside_labelled_ones_is_decoration() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![
            outline(0.0, 0.0, 100.0, 50.0),
            outline(0.0, 200.0, 100.0, 250.0),
            outline(40.0, 300.0, 52.0, 312.0),
        ],
        vec![
            connector((50.0, 50.0), (50.0, 200.0)),
            connector((50.0, 250.0), (46.0, 300.0)),
        ],
        vec![label(50.0, 25.0, "from"), label(50.0, 225.0, "to")],
    )
    .expect("graph");

    assert_eq!(graph.nodes.len(), 2, "the dot is decoration: {:?}", graph.nodes);
}

#[test]
fn two_boxes_and_a_line_make_an_edge() {
    let graph = assemble(
        Some("g".into()),
        (400.0, 400.0),
        vec![outline(0.0, 0.0, 100.0, 50.0), outline(0.0, 200.0, 100.0, 250.0)],
        vec![connector((50.0, 50.0), (50.0, 200.0))],
        vec![label(50.0, 25.0, "top"), label(50.0, 225.0, "bottom")],
    )
    .expect("graph");

    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.nodes[0].label, "top");
    assert_eq!(graph.nodes[1].label, "bottom");
    assert_eq!(graph.edges.len(), 1);
    assert_eq!((graph.edges[0].from, graph.edges[0].to), (0, 1));
}

#[test]
fn shapes_without_connectors_are_not_a_graph() {
    assert!(
        assemble(
            None,
            (400.0, 400.0),
            vec![outline(0.0, 0.0, 100.0, 50.0), outline(0.0, 200.0, 100.0, 250.0)],
            Vec::new(),
            Vec::new(),
        )
        .is_none()
    );
}

#[test]
fn background_panel_is_not_a_node() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![
            outline(0.0, 0.0, 400.0, 400.0),
            outline(0.0, 0.0, 100.0, 50.0),
            outline(0.0, 200.0, 100.0, 250.0),
        ],
        vec![connector((50.0, 50.0), (50.0, 200.0))],
        Vec::new(),
    )
    .expect("graph");

    assert_eq!(graph.nodes.len(), 2);
}

#[test]
fn shapes_no_connector_reaches_are_kept() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![
            outline(0.0, 0.0, 100.0, 50.0),
            // Sorts second, and no connector touches it.
            outline(0.0, 100.0, 100.0, 150.0),
            outline(0.0, 200.0, 100.0, 250.0),
        ],
        vec![connector((50.0, 50.0), (50.0, 200.0))],
        Vec::new(),
    )
    .expect("graph");

    assert_eq!(graph.nodes.len(), 3);
    assert_eq!((graph.edges[0].from, graph.edges[0].to), (0, 2));
}

#[test]
fn duplicate_outlines_and_edges_collapse() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![
            outline(0.0, 0.0, 100.0, 50.0),
            outline(0.0, 0.0, 100.0, 50.0),
            outline(0.0, 200.0, 100.0, 250.0),
        ],
        vec![
            connector((50.0, 50.0), (50.0, 200.0)),
            connector((50.0, 50.0), (50.0, 200.0)),
        ],
        Vec::new(),
    )
    .expect("graph");

    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);
}

#[test]
fn text_outside_every_shape_can_label_an_edge() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![outline(0.0, 0.0, 100.0, 50.0), outline(0.0, 200.0, 100.0, 250.0)],
        vec![connector((50.0, 50.0), (50.0, 200.0))],
        vec![label(52.0, 126.0, "yes"), label(390.0, 390.0, "footer")],
    )
    .expect("graph");

    assert_eq!(graph.edges[0].label.as_deref(), Some("yes"));
    assert!(graph.nodes.iter().all(|n| n.label.is_empty()));
}

#[test]
fn label_attaches_to_the_innermost_containing_shape() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![
            // A panel that is large but still under the background cutoff.
            outline(0.0, 0.0, 200.0, 300.0),
            outline(10.0, 10.0, 100.0, 60.0),
            outline(10.0, 200.0, 100.0, 250.0),
        ],
        vec![connector((50.0, 60.0), (50.0, 200.0))],
        vec![label(50.0, 30.0, "inner")],
    )
    .expect("graph");

    let inner = graph.nodes.iter().find(|n| n.label == "inner");
    assert!(inner.is_some(), "label went to the panel instead of the box");
}

fn arrowhead(cx: f32, cy: f32) -> Outline {
    outline(cx - 4.0, cy - 5.0, cx + 4.0, cy + 5.0)
}

/// A renderer draws an arrow as a small filled shape on the end of the
/// line. It is not a node, and the line has to reach past it.
#[test]
fn an_arrowhead_is_not_a_node_and_sets_direction() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![
            outline(0.0, 0.0, 100.0, 50.0),
            arrowhead(50.0, 194.0),
            outline(0.0, 200.0, 100.0, 250.0),
        ],
        vec![connector((50.0, 50.0), (50.0, 189.0))],
        vec![label(50.0, 25.0, "top"), label(50.0, 225.0, "bottom")],
    )
    .expect("graph");

    assert_eq!(graph.nodes.len(), 2, "arrowhead became a node: {:?}", graph.nodes);
    assert_eq!(graph.nodes[0].label, "top");
    assert_eq!(graph.nodes[1].label, "bottom");
    assert_eq!((graph.edges[0].from, graph.edges[0].to), (0, 1));
    assert!(!graph.edges[0].bidirectional);
}

#[test]
fn an_arrowhead_at_the_start_reverses_the_edge() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![
            outline(0.0, 0.0, 100.0, 50.0),
            arrowhead(50.0, 56.0),
            outline(0.0, 200.0, 100.0, 250.0),
        ],
        vec![connector((50.0, 61.0), (50.0, 200.0))],
        Vec::new(),
    )
    .expect("graph");

    assert_eq!((graph.edges[0].from, graph.edges[0].to), (1, 0));
}

#[test]
fn arrowheads_at_both_ends_are_bidirectional() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![
            outline(0.0, 0.0, 100.0, 50.0),
            arrowhead(50.0, 56.0),
            arrowhead(50.0, 194.0),
            outline(0.0, 200.0, 100.0, 250.0),
        ],
        vec![connector((50.0, 61.0), (50.0, 189.0))],
        Vec::new(),
    )
    .expect("graph");

    assert_eq!(graph.nodes.len(), 2);
    assert!(graph.edges[0].bidirectional);
}

/// A `doublecircle` is one node drawn as two rings.
#[test]
fn a_double_border_is_one_node() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![
            outline(0.0, 0.0, 100.0, 50.0),
            outline(0.0, 200.0, 100.0, 250.0),
            outline(4.0, 204.0, 96.0, 246.0),
        ],
        vec![connector((50.0, 50.0), (50.0, 200.0))],
        vec![label(50.0, 225.0, "done")],
    )
    .expect("graph");

    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.nodes[1].label, "done");
    assert_eq!((graph.edges[0].from, graph.edges[0].to), (0, 1));
}

/// Graphviz fills the inner disc of a `doublecircle` and leaves the outer
/// ring unpainted, so keeping the outer geometry has to keep the inner
/// paint or the node loses its colour.
#[test]
fn a_double_border_keeps_the_styling_of_both_rings() {
    let mut outer = outline(0.0, 200.0, 100.0, 250.0);
    outer.fill = None;
    outer.stroke = Some("#000000".to_string());
    let mut inner = outline(4.0, 204.0, 96.0, 246.0);
    inner.fill = Some("#fb8072".to_string());
    inner.stroke = None;

    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![outline(0.0, 0.0, 100.0, 50.0), outer, inner],
        vec![connector((50.0, 50.0), (50.0, 200.0))],
        Vec::new(),
    )
    .expect("graph");

    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.nodes[1].fill.as_deref(), Some("#fb8072"));
    assert_eq!(graph.nodes[1].stroke.as_deref(), Some("#000000"));
}

/// A panel encloses its boxes too, but it is far larger, so it must not be
/// collapsed into them.
#[test]
fn an_enclosing_panel_is_a_container_not_a_node() {
    let graph = assemble(
        None,
        (400.0, 400.0),
        vec![
            outline(0.0, 0.0, 200.0, 300.0),
            outline(10.0, 10.0, 100.0, 60.0),
            outline(10.0, 200.0, 100.0, 250.0),
        ],
        vec![connector((50.0, 60.0), (50.0, 200.0))],
        Vec::new(),
    )
    .expect("graph");

    // The panel groups the two boxes, so it is a container: reporting it
    // would both invent a node and give the connector a third thing to
    // land on. It is still not a double border, which is what would
    // happen if it were merged into the shape it encloses.
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);
}

// The tests below call `find_containers` directly rather than through
// `assemble`, so that each states one classification rule and fails for one
// reason. `assemble` reaches the same verdicts, and
// `an_enclosing_panel_is_a_container_not_a_node` covers it end to end. ~keep

/// Two of a node's own incoming arrowheads, drawn just inside its border,
/// satisfy `Rect::encloses` exactly as two real members would. They must
/// not count: excluding them leaves zero members, short of the container
/// threshold.
#[test]
fn a_container_test_excludes_arrowheads_from_its_member_count() {
    let outlines = vec![
        outline(0.0, 0.0, 100.0, 100.0),
        outline(5.0, 5.0, 15.0, 15.0),
        outline(80.0, 80.0, 90.0, 90.0),
    ];
    let arrowheads = vec![false, true, true];
    let owned: Vec<Vec<&Label>> = vec![Vec::new(), Vec::new(), Vec::new()];

    assert_eq!(
        find_containers(&outlines, &arrowheads, &owned, &nowhere(3), 4.0),
        vec![false, false, false],
        "two arrowheads inside a node's border are not two members grouped by it"
    );
}

/// A UML class box or a BPMN task carries its own label alongside the
/// compartments or markers it draws inside itself. Nothing connects to
/// those, so the enclosure is a node.
#[test]
fn a_labelled_enclosure_of_unconnected_detail_is_a_node() {
    let outlines = vec![
        outline(0.0, 0.0, 100.0, 100.0),
        outline(10.0, 10.0, 40.0, 40.0),
        outline(60.0, 60.0, 90.0, 90.0),
    ];
    let arrowheads = vec![false, false, false];
    let caption = label(50.0, 50.0, "ClassName");
    let owned: Vec<Vec<&Label>> = vec![vec![&caption], Vec::new(), Vec::new()];

    assert_eq!(
        find_containers(&outlines, &arrowheads, &owned, &nowhere(3), 4.0),
        vec![false, false, false],
        "compartments nothing connects to are interior detail, not grouped members"
    );
}

/// A Graphviz cluster is captioned inside its own border, so it owns a
/// label exactly as a class box does. What separates them is that its
/// members carry the diagram's connectors.
#[test]
fn a_labelled_enclosure_grouping_connected_shapes_is_a_container() {
    let outlines = vec![
        outline(0.0, 0.0, 100.0, 100.0),
        outline(10.0, 10.0, 40.0, 40.0),
        outline(60.0, 60.0, 90.0, 90.0),
    ];
    let arrowheads = vec![false, false, false];
    let caption = label(50.0, 5.0, "Ingest");
    let owned: Vec<Vec<&Label>> = vec![vec![&caption], Vec::new(), Vec::new()];
    // One connector joining the two members, landing on each well inside
    // the enclosure rather than on its rim.
    let landings = vec![Vec::new(), vec![(25.0, 40.0)], vec![(75.0, 60.0)]];

    assert_eq!(
        find_containers(&outlines, &arrowheads, &owned, &landings, 4.0),
        vec![true, false, false],
        "a captioned box grouping two connected shapes is a cluster, not a node"
    );
}

/// A class box's compartments run its full width, so an association
/// arriving at the class's own border is at zero distance from whichever
/// compartment reaches that height. Landing on the rim is not being
/// grouped, or every UML class in the world becomes a container.
#[test]
fn a_labelled_enclosure_whose_members_are_touched_only_at_its_rim_is_a_node() {
    let outlines = vec![
        outline(0.0, 0.0, 100.0, 100.0),
        outline(0.0, 10.0, 100.0, 40.0),
        outline(0.0, 60.0, 100.0, 90.0),
    ];
    let arrowheads = vec![false, false, false];
    let caption = label(50.0, 5.0, "Order");
    let owned: Vec<Vec<&Label>> = vec![vec![&caption], Vec::new(), Vec::new()];
    // Two associations arriving on the class's left border, each landing on
    // a different full-width compartment.
    let landings = vec![Vec::new(), vec![(0.0, 20.0)], vec![(0.0, 70.0)]];

    assert_eq!(
        find_containers(&outlines, &arrowheads, &owned, &landings, 4.0),
        vec![false, false, false],
        "compartments touched on the enclosure's rim are interior detail"
    );
}

/// The baseline the two tests above are contrasted with: an unlabelled
/// enclosure of two ordinary (non-arrowhead) shapes is still a container.
#[test]
fn a_container_test_still_condemns_an_unlabelled_enclosure_of_two_real_shapes() {
    let outlines = vec![
        outline(0.0, 0.0, 100.0, 100.0),
        outline(10.0, 10.0, 40.0, 40.0),
        outline(60.0, 60.0, 90.0, 90.0),
    ];
    let arrowheads = vec![false, false, false];
    let owned: Vec<Vec<&Label>> = vec![Vec::new(), Vec::new(), Vec::new()];

    assert_eq!(
        find_containers(&outlines, &arrowheads, &owned, &nowhere(3), 4.0),
        vec![true, false, false]
    );
}

#[test]
fn a_stroke_inside_one_shape_is_not_a_self_loop() {
    assert!(
        assemble(
            None,
            (400.0, 400.0),
            vec![outline(0.0, 0.0, 100.0, 50.0), outline(0.0, 200.0, 100.0, 250.0)],
            vec![connector((10.0, 10.0), (90.0, 40.0))],
            Vec::new(),
        )
        .is_none()
    );
}

/// GH#1420: a bar chart's gridlines, drawn across the whole plot, land
/// exactly on the first and last bar and read as a connector between
/// them unless the family they belong to is recognised as chrome. Four
/// bars, no genuine connector between any of them, and three gridlines
/// sharing one colour, evenly spaced and each spanning the same width —
/// nothing here should survive as a graph at all.
#[test]
fn bar_chart_gridlines_crossing_the_bars_are_not_a_graph() {
    let bars = vec![
        outline(120.0, 180.0, 220.0, 380.0),
        outline(250.0, 240.0, 350.0, 380.0),
        outline(380.0, 130.0, 480.0, 380.0),
        outline(510.0, 210.0, 610.0, 380.0),
    ];
    let gridlines = vec![
        styled_connector((120.0, 300.0), (620.0, 300.0), "#cccccc"),
        styled_connector((120.0, 220.0), (620.0, 220.0), "#cccccc"),
        styled_connector((120.0, 140.0), (620.0, 140.0), "#cccccc"),
    ];

    assert!(
        assemble(None, (700.0, 450.0), bars, gridlines, Vec::new()).is_none(),
        "chart gridlines that cross the bars must not read as a graph"
    );
}

/// The gridline family is removed on sight, not just when it is the only
/// thing present: a real connector elsewhere in the drawing, in a
/// different colour and direction, survives the same pass untouched.
#[test]
fn gridlines_are_dropped_but_a_genuine_edge_beside_them_survives() {
    let bars = vec![
        outline(120.0, 180.0, 220.0, 380.0),
        outline(250.0, 240.0, 350.0, 380.0),
        outline(380.0, 130.0, 480.0, 380.0),
        outline(510.0, 210.0, 610.0, 380.0),
    ];
    let extra_nodes = vec![outline(630.0, 0.0, 700.0, 50.0), outline(630.0, 60.0, 700.0, 110.0)];
    let gridlines = vec![
        styled_connector((120.0, 300.0), (620.0, 300.0), "#cccccc"),
        styled_connector((120.0, 220.0), (620.0, 220.0), "#cccccc"),
        styled_connector((120.0, 140.0), (620.0, 140.0), "#cccccc"),
    ];
    let real_edge = styled_connector((665.0, 50.0), (665.0, 60.0), "#333333");

    let mut connectors = gridlines;
    connectors.push(real_edge);
    let mut outlines = bars;
    outlines.extend(extra_nodes);

    let graph = assemble(None, (700.0, 450.0), outlines, connectors, Vec::new()).expect("the real edge survives");

    assert_eq!(
        graph.edges.len(),
        1,
        "only the non-gridline connector is an edge: {:?}",
        graph.edges
    );
    assert_eq!(graph.edges[0].stroke.as_deref(), Some("#333333"));
}

fn dashed_connector(start: (f32, f32), end: (f32, f32), stroke: &str) -> Connector {
    Connector {
        dashed: true,
        ..styled_connector(start, end, stroke)
    }
}

#[test]
fn a_regular_grid_mixing_dashed_and_solid_lines_is_one_family() {
    let lines = vec![
        dashed_connector((120.0, 300.0), (620.0, 300.0), "#cccccc"),
        styled_connector((120.0, 260.0), (620.0, 260.0), "#cccccc"),
        dashed_connector((120.0, 220.0), (620.0, 220.0), "#cccccc"),
        styled_connector((120.0, 180.0), (620.0, 180.0), "#cccccc"),
    ];
    assert_eq!(matching::find_gridlines(&lines, 5.0), [true; 4]);
}

#[test]
fn a_regular_dashed_grid_is_found_beside_an_irregular_solid_line_of_its_colour() {
    let lines = vec![
        dashed_connector((120.0, 300.0), (620.0, 300.0), "#cccccc"),
        dashed_connector((120.0, 220.0), (620.0, 220.0), "#cccccc"),
        dashed_connector((120.0, 140.0), (620.0, 140.0), "#cccccc"),
        styled_connector((120.0, 190.0), (400.0, 190.0), "#cccccc"),
    ];
    assert_eq!(matching::find_gridlines(&lines, 5.0), [true, true, true, false]);
}

/// A straight top-to-bottom chain of linked shapes is the case a naive
/// "regularly spaced, same-direction, same-style strokes" rule would
/// wrongly condemn: consecutive edges in a vertical flowchart are
/// axis-aligned, share a colour and are evenly spaced because the nodes
/// are. What tells them apart from gridlines is span (each edge here
/// covers only the short run between its own two nodes) and collinearity
/// (they share their run-axis position exactly, which gridlines never
/// do). All three edges must survive with their exact endpoints.
#[test]
fn a_straight_vertical_chain_is_not_mistaken_for_gridlines() {
    let nodes = vec![
        outline(0.0, 0.0, 100.0, 50.0),
        outline(0.0, 100.0, 100.0, 150.0),
        outline(0.0, 200.0, 100.0, 250.0),
        outline(0.0, 300.0, 100.0, 350.0),
    ];
    let links = vec![
        styled_connector((50.0, 50.0), (50.0, 100.0), "#000000"),
        styled_connector((50.0, 150.0), (50.0, 200.0), "#000000"),
        styled_connector((50.0, 250.0), (50.0, 300.0), "#000000"),
    ];

    let graph = assemble(None, (400.0, 400.0), nodes, links, Vec::new()).expect("a real chain is a graph");

    let edges: Vec<(usize, usize)> = graph.edges.iter().map(|e| (e.from, e.to)).collect();
    assert_eq!(
        edges,
        vec![(0, 1), (1, 2), (2, 3)],
        "a real vertical chain loses no edges"
    );
}
