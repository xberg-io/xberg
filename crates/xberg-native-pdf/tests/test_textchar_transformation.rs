//! Tests for TextChar transformation properties.
//!
//! These tests verify the extraction and usage of:
//! - origin_x, origin_y (baseline position)
//! - rotation_degrees (rotation angle)
//! - advance_width (horizontal advance)
//! - matrix (full transformation matrix)

use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::text_block::{Color, FontWeight, TextChar};

/// Helper to create a TextChar with specific transformation properties.
fn create_char_with_transform(c: char, x: f32, y: f32, rotation: f32, matrix: Option<[f32; 6]>) -> TextChar {
    let bbox = Rect::new(x, y, 10.0, 12.0);
    TextChar {
        char: c,
        bbox,
        font_name: "Helvetica".into(),
        font_size: 12.0,
        font_weight: FontWeight::Normal,
        is_italic: false,
        is_monospace: false,
        color: Color::black(),
        mcid: None,
        origin_x: x,
        origin_y: y,
        rotation_degrees: rotation,
        advance_width: 10.0,
        rendered_advance: 10.0,
        ascent: 0.95,
        descent: -0.35,
        matrix,
    }
}

#[test]
fn test_textchar_no_rotation() {
    let char = create_char_with_transform('A', 100.0, 200.0, 0.0, None);

    assert!(!char.is_rotated());
    assert_eq!(char.rotation_degrees, 0.0);
    assert_eq!(char.origin_x, 100.0);
    assert_eq!(char.origin_y, 200.0);
}

#[test]
fn test_textchar_90_degree_rotation() {
    let matrix = [0.0, 1.0, -1.0, 0.0, 100.0, 200.0];
    let char = create_char_with_transform('A', 100.0, 200.0, 90.0, Some(matrix));

    assert!(char.is_rotated());
    assert!((char.rotation_degrees - 90.0).abs() < 0.1);
}

#[test]
fn test_textchar_180_degree_rotation() {
    let matrix = [-1.0, 0.0, 0.0, -1.0, 100.0, 200.0];
    let char = create_char_with_transform('A', 100.0, 200.0, 180.0, Some(matrix));

    assert!(char.is_rotated());
    assert!((char.rotation_degrees - 180.0).abs() < 0.1);
}

#[test]
fn test_textchar_270_degree_rotation() {
    let matrix = [0.0, -1.0, 1.0, 0.0, 100.0, 200.0];
    let char = create_char_with_transform('A', 100.0, 200.0, 270.0, Some(matrix));

    assert!(char.is_rotated());
    assert!((char.rotation_degrees - 270.0).abs() < 0.1);
}

#[test]
fn test_textchar_45_degree_rotation() {
    let cos45 = std::f32::consts::FRAC_1_SQRT_2;
    let sin45 = std::f32::consts::FRAC_1_SQRT_2;
    let matrix = [cos45, sin45, -sin45, cos45, 100.0, 200.0];
    let char = create_char_with_transform('A', 100.0, 200.0, 45.0, Some(matrix));

    assert!(char.is_rotated());
    assert!((char.rotation_degrees - 45.0).abs() < 0.1);
}

#[test]
fn test_textchar_rotation_radians() {
    let char = create_char_with_transform('A', 100.0, 200.0, 90.0, None);

    let radians = char.rotation_radians();
    assert!((radians - std::f32::consts::FRAC_PI_2).abs() < 0.001);
}

#[test]
fn test_textchar_rotation_threshold() {
    let char = create_char_with_transform('A', 100.0, 200.0, 0.005, None);
    assert!(!char.is_rotated());

    let char2 = create_char_with_transform('A', 100.0, 200.0, 0.02, None);
    assert!(char2.is_rotated());
}

#[test]
fn test_textchar_origin_coordinates() {
    let char = create_char_with_transform('A', 123.45, 678.90, 0.0, None);

    assert_eq!(char.origin_x, 123.45);
    assert_eq!(char.origin_y, 678.90);
}

#[test]
fn test_textchar_advance_width() {
    let bbox = Rect::new(100.0, 200.0, 15.0, 12.0);
    let char = TextChar {
        char: 'W',
        bbox,
        font_name: "Helvetica".into(),
        font_size: 12.0,
        font_weight: FontWeight::Normal,
        is_italic: false,
        is_monospace: false,
        color: Color::black(),
        mcid: None,
        origin_x: 100.0,
        origin_y: 200.0,
        rotation_degrees: 0.0,
        advance_width: 15.5,
        rendered_advance: 15.5,
        ascent: 0.95,
        descent: -0.35,
        matrix: None,
    };

    assert_eq!(char.advance_width, 15.5);
}

#[test]
fn test_textchar_matrix_access() {
    let matrix = [1.0, 0.0, 0.0, 1.0, 100.0, 200.0];
    let char = create_char_with_transform('A', 100.0, 200.0, 0.0, Some(matrix));

    let retrieved = char.get_matrix();
    assert_eq!(retrieved, matrix);
}

#[test]
fn test_textchar_reconstructed_matrix() {
    let char = create_char_with_transform('A', 100.0, 200.0, 0.0, None);
    let reconstructed = char.get_matrix();

    assert!((reconstructed[0] - 1.0).abs() < 0.001);
    assert!((reconstructed[1] - 0.0).abs() < 0.001);
    assert!((reconstructed[4] - 100.0).abs() < 0.001);
    assert!((reconstructed[5] - 200.0).abs() < 0.001);
}

#[test]
fn test_textchar_with_matrix_builder() {
    let bbox = Rect::new(100.0, 200.0, 10.0, 12.0);
    let char = TextChar {
        char: 'A',
        bbox,
        font_name: "Helvetica".into(),
        font_size: 12.0,
        font_weight: FontWeight::Normal,
        is_italic: false,
        is_monospace: false,
        color: Color::black(),
        mcid: None,
        origin_x: 100.0,
        origin_y: 200.0,
        rotation_degrees: 0.0,
        advance_width: 10.0,
        rendered_advance: 10.0,
        ascent: 0.95,
        descent: -0.35,
        matrix: None,
    };

    let matrix = [1.0, 0.0, 0.0, 1.0, 50.0, 50.0];
    let char_with_matrix = char.with_matrix(matrix);

    assert!(char_with_matrix.matrix.is_some());
    assert_eq!(char_with_matrix.get_matrix(), matrix);
}

#[test]
fn test_textchar_simple_constructor() {
    let bbox = Rect::new(100.0, 200.0, 10.0, 12.0);
    let char = TextChar::simple('X', bbox, "Helvetica".to_string(), 12.0);

    assert_eq!(char.char, 'X');
    assert_eq!(char.origin_x, bbox.x);
    assert_eq!(char.origin_y, bbox.y);
    assert_eq!(char.rotation_degrees, 0.0);
    assert_eq!(char.advance_width, bbox.width);
    assert!(char.matrix.is_none());
    assert!(!char.is_rotated());
}

#[test]
fn test_textchar_negative_rotation() {
    let char = create_char_with_transform('A', 100.0, 200.0, -45.0, None);

    assert!(char.is_rotated());
    assert_eq!(char.rotation_degrees, -45.0);
}

#[test]
fn test_rotation_calculation_from_matrix() {
    let angle_degrees = 30.0_f32;
    let angle_radians = angle_degrees.to_radians();
    let cos_a = angle_radians.cos();
    let sin_a = angle_radians.sin();

    let matrix = [cos_a, sin_a, -sin_a, cos_a, 0.0, 0.0];

    let calculated_rotation = matrix[1].atan2(matrix[0]).to_degrees();

    assert!((calculated_rotation - angle_degrees).abs() < 0.01);
}

#[test]
fn test_scaled_and_rotated_matrix() {
    let scale = 2.0_f32;
    let angle_radians = std::f32::consts::FRAC_PI_4;
    let cos_a = angle_radians.cos();
    let sin_a = angle_radians.sin();

    let matrix = [
        scale * cos_a,
        scale * sin_a,
        -scale * sin_a,
        scale * cos_a,
        100.0,
        200.0,
    ];

    let calculated_rotation = matrix[1].atan2(matrix[0]).to_degrees();

    assert!((calculated_rotation - 45.0).abs() < 0.01);
}
