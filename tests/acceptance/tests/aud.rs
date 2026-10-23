//! AT-AUD-* acceptance tests. Generated from docs/ACCEPTANCE.md;
//! remove `#[ignore]` when implementing each test in its milestone.

/// AT-AUD-01: Every committed tx has exactly one event
/// Pass: Count(events committed) == count(Committed results); no orphan state
#[test]
#[ignore = "M3"]
fn at_aud_01() {
    todo!("AT-AUD-01");
}

/// AT-AUD-02: `explain tx`
/// Pass: Answers the six questions (SPEC §7.3); secret fields redacted
#[test]
#[ignore = "M3"]
fn at_aud_02() {
    todo!("AT-AUD-02");
}

/// AT-AUD-03: Flip one byte in an event
/// Pass: `verify` reports first bad seq
#[test]
#[ignore = "M3"]
fn at_aud_03() {
    todo!("AT-AUD-03");
}

/// AT-AUD-04: Delete a middle event
/// Pass: `verify` reports gap
#[test]
#[ignore = "M3"]
fn at_aud_04() {
    todo!("AT-AUD-04");
}

/// AT-AUD-05: Reorder two events
/// Pass: `verify` reports chain break
#[test]
#[ignore = "M3"]
fn at_aud_05() {
    todo!("AT-AUD-05");
}

/// AT-AUD-06: Edit state without event
/// Pass: `verify --state` reports state/history mismatch
#[test]
#[ignore = "M3"]
fn at_aud_06() {
    todo!("AT-AUD-06");
}

/// AT-AUD-07: Truncate history after last anchored checkpoint
/// Pass: `verify --anchors` reports truncation
#[test]
#[ignore = "M3"]
fn at_aud_07() {
    todo!("AT-AUD-07");
}

/// AT-AUD-08: Checkpoint signatures
/// Pass: Both Ed25519 and ML-DSA-65 verify; either failing ⇒ invalid
#[test]
#[ignore = "M3"]
fn at_aud_08() {
    todo!("AT-AUD-08");
}

/// AT-AUD-09: Explanation labels intent as claimed
/// Pass: Output separates "claimed" from "verified"
#[test]
#[ignore = "M3"]
fn at_aud_09() {
    todo!("AT-AUD-09");
}

/// AT-AUD-10: Rejected attempts recorded
/// Pass: Rejection events present, rate limit honoured
#[test]
#[ignore = "M3"]
fn at_aud_10() {
    todo!("AT-AUD-10");
}
