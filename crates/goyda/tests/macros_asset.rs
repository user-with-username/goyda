//! Tests for the `asset!`/`asset_ref!` macros in `src/macros.rs`.
//!
//! Both resolve paths relative to this crate's own `assets/` directory at
//! compile time, so they're exercised against the `assets/test_fixture.txt`
//! fixture checked in alongside these tests.

use goyda::{asset, asset_ref};


#[test]
fn asset_ref_does_not_embed_bytes() {
    let a = asset_ref!("test_fixture.txt");
    assert_eq!(a.path(), "test_fixture.txt");
    assert_eq!(a.bytes(), None);
}
