//! AT-PROT-* acceptance tests. Generated from docs/ACCEPTANCE.md;
//! remove `#[ignore]` when implementing each test in its milestone.

/// AT-PROT-01: Typed JSON round trip
/// Pass: decode(encode(r)) == r for generated requests
#[test]
#[ignore = "M1"]
fn at_prot_01() {
    todo!("AT-PROT-01");
}

/// AT-PROT-02: Reject unknown field / trailing bytes / duplicate key
/// Pass: `Malformed`, nothing executed
#[test]
#[ignore = "M1"]
fn at_prot_02() {
    todo!("AT-PROT-02");
}

/// AT-PROT-03: Same request typed vs binary
/// Pass: Identical results and identical history events (except `time`)
#[test]
#[ignore = "M1"]
fn at_prot_03() {
    todo!("AT-PROT-03");
}

/// AT-PROT-04: Unknown protocol version
/// Pass: `Malformed{unsupported_version}`
#[test]
#[ignore = "M1"]
fn at_prot_04() {
    todo!("AT-PROT-04");
}

/// AT-PROT-05: 257 ops in one request
/// Pass: `LimitExceeded`
#[test]
#[ignore = "M1"]
fn at_prot_05() {
    todo!("AT-PROT-05");
}
