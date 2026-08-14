//! Tests for `src/style.rs`.

use goyda_utils::style::{resolve_length, resolve_spacing, StyleValue, SPACING};

#[test]
fn spacing_scale_starts_at_zero_and_has_thirteen_steps() {
    assert_eq!(SPACING.len(), 13);
    assert_eq!(SPACING[0], 0.0);
    assert_eq!(SPACING[12], 80.0);
}

#[test]
fn resolve_spacing_looks_up_a_valid_index() {
    assert_eq!(resolve_spacing(0), Some(0.0));
    assert_eq!(resolve_spacing(3), Some(8.0));
    assert_eq!(resolve_spacing(12), Some(80.0));
}

#[test]
fn resolve_spacing_is_none_out_of_range() {
    assert_eq!(resolve_spacing(13), None);
    assert_eq!(resolve_spacing(999), None);
}

#[test]
fn resolve_length_reads_a_literal_length_directly() {
    assert_eq!(resolve_length(&StyleValue::Length(42.0)), Some(42.0));
}

#[test]
fn resolve_length_looks_up_a_spacing_index() {
    assert_eq!(resolve_length(&StyleValue::Spacing(3)), Some(8.0));
}

#[test]
fn resolve_length_is_none_for_an_out_of_range_spacing_index() {
    assert_eq!(resolve_length(&StyleValue::Spacing(999)), None);
}

#[test]
fn resolve_length_is_none_for_non_length_values() {
    assert_eq!(resolve_length(&StyleValue::Number(1.0)), None);
    assert_eq!(resolve_length(&StyleValue::Bool(true)), None);
}
