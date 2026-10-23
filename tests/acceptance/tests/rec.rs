//! AT-REC-* acceptance tests. Generated from docs/ACCEPTANCE.md;
//! remove `#[ignore]` when implementing each test in its milestone.

/// AT-REC-01: Kill process at random points during 10 000 commits (100 runs)
/// Pass: After restart: every acknowledged commit present, no partial tx, chain verifies
#[test]
#[ignore = "M3"]
fn at_rec_01() {
    todo!("AT-REC-01");
}

/// AT-REC-02: Torn write at end of storage
/// Pass: Recovery discards incomplete batch, verify passes
#[test]
#[ignore = "M3"]
fn at_rec_02() {
    todo!("AT-REC-02");
}

/// AT-REC-03: Restart after clean shutdown
/// Pass: Final checkpoint present and valid
#[test]
#[ignore = "M3"]
fn at_rec_03() {
    todo!("AT-REC-03");
}

/// AT-REC-04: Recovery time for 10 GB store
/// Pass: ≤ 60 s (NFR-4)
#[test]
#[ignore = "M3"]
fn at_rec_04() {
    todo!("AT-REC-04");
}
