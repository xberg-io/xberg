use super::*;
use std::cell::Cell;

const TILE_LIMITS: StripLimits = StripLimits {
    max_dimension: 4096,
    max_lookback: 2048,
    max_tiles: 8,
    max_tile_pixels: 4096 * 4096,
    max_total_pixels: 40_000_000,
};
const SYNTHETIC_BANDS: [Range<u32>; 3] = [3805..4090, 7825..8111, 11846..12132];

fn plan() -> StripPlan {
    plan_strips(1275, 12500, TILE_LIMITS, &SYNTHETIC_BANDS, || false).expect("certified synthetic strips")
}

#[test]
fn strips_cover_all_source_rows_once_with_bounded_dimensions() {
    let plan = plan();
    assert_eq!(plan.rows, [0..3947, 3947..7968, 7968..11989, 11989..12500]);
    assert_eq!(plan.total_pixels, 15_937_500);
    assert_eq!(plan.rows.iter().map(|rows| rows.len()).max(), Some(4021));
}

#[test]
fn missing_or_invalid_certificates_do_not_produce_a_partial_plan() {
    assert_eq!(
        plan_strips(1275, 12500, TILE_LIMITS, &[], || false),
        Err(Refusal::Ambiguous)
    );
    let narrow = StripLimits {
        max_lookback: 128,
        ..TILE_LIMITS
    };
    assert_eq!(
        plan_strips(1275, 12500, narrow, &SYNTHETIC_BANDS, || false),
        Err(Refusal::Ambiguous)
    );
    assert_eq!(
        plan_strips(1275, 12500, TILE_LIMITS, &[1800..1840, 1810..1880], || false),
        Err(Refusal::Bounds)
    );
}

#[test]
fn in_cap_and_horizontal_oversize_defer_without_tiling() {
    assert_eq!(
        plan_strips(1275, 1650, TILE_LIMITS, &[], || false),
        Err(Refusal::Unneeded)
    );
    assert_eq!(
        plan_strips(4097, 12500, TILE_LIMITS, &[], || false),
        Err(Refusal::Bounds)
    );
    assert_eq!(plan_strips(0, 12500, TILE_LIMITS, &[], || false), Err(Refusal::Bounds));
}

#[test]
fn tile_count_and_pixel_budget_include_the_actual_planned_strips() {
    let exact = plan().total_pixels;
    let limits = StripLimits {
        max_total_pixels: exact - 1,
        ..TILE_LIMITS
    };
    assert_eq!(
        plan_strips(1275, 12500, limits, &SYNTHETIC_BANDS, || false),
        Err(Refusal::Budget)
    );
    let limits = StripLimits {
        max_total_pixels: exact,
        ..TILE_LIMITS
    };
    assert_eq!(
        plan_strips(1275, 12500, limits, &SYNTHETIC_BANDS, || false)
            .unwrap()
            .total_pixels,
        exact
    );
    let bands = [3020..3060, 6040..6080, 9060..9100, 12080..12120];
    let limits = StripLimits {
        max_tile_pixels: 1275 * 3072,
        ..TILE_LIMITS
    };
    let small_tiles = plan_strips(1275, 12500, limits, &bands, || false).unwrap();
    assert_eq!(small_tiles.rows.len(), 5);
    assert_eq!(small_tiles.rows.iter().map(|rows| rows.len()).max(), Some(3040));
    assert_eq!(
        plan_strips(1275, 12500, StripLimits { max_tiles: 4, ..limits }, &bands, || false),
        Err(Refusal::Budget)
    );
}

#[test]
fn planning_checks_cancellation_after_certificates_and_between_strips() {
    let checks = Cell::new(0);
    let result = plan_strips(1275, 12500, TILE_LIMITS, &SYNTHETIC_BANDS, || {
        let previous = checks.get();
        checks.set(previous + 1);
        previous == 5
    });
    assert_eq!(result, Err(Refusal::Cancelled));
    assert_eq!(checks.get(), 6);
}
