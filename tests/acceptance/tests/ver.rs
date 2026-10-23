//! AT-VER-* acceptance tests. Generated from docs/ACCEPTANCE.md;
//! remove `#[ignore]` when implementing each test in its milestone.

/// AT-VER-01: Required verifier rejects
/// Pass: Abort, `VerifierRejected{id}`, evidence recorded
#[test]
#[ignore = "M4"]
fn at_ver_01() {
    todo!("AT-VER-01");
}

/// AT-VER-02: Advisory verifier rejects
/// Pass: Commit proceeds, verdict recorded
#[test]
#[ignore = "M4"]
fn at_ver_02() {
    todo!("AT-VER-02");
}

/// AT-VER-03: Verifier timeout
/// Pass: Treated as reject for required verifiers
#[test]
#[ignore = "M4"]
fn at_ver_03() {
    todo!("AT-VER-03");
}

/// AT-VER-04: State changes between verify and commit
/// Pass: Commit aborts with `Conflict` (no TOCTOU)
#[test]
#[ignore = "M4"]
fn at_ver_04() {
    todo!("AT-VER-04");
}

/// AT-VER-05: Verifier attempts to mutate proposal
/// Pass: Impossible by API (compile-time) and write set hash unchanged
#[test]
#[ignore = "M4"]
fn at_ver_05() {
    todo!("AT-VER-05");
}

/// AT-VER-06: Verifier accepts invariant-violating tx
/// Pass: Deterministic invariant still rejects
#[test]
#[ignore = "M4"]
fn at_ver_06() {
    todo!("AT-VER-06");
}

/// AT-VER-07: Seeded-fault corpus (100 wrong ops)
/// Pass: Report detection rate for reference verifier (measured)
#[test]
#[ignore = "M4"]
fn at_ver_07() {
    todo!("AT-VER-07");
}
