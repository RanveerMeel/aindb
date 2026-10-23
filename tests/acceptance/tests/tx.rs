//! AT-TX-* acceptance tests. Generated from docs/ACCEPTANCE.md;
//! remove `#[ignore]` when implementing each test in its milestone.

/// AT-TX-01: Transfer A→B
/// Pass: Both balances updated, one event, one seq
#[test]
#[ignore = "M2"]
fn at_tx_01() {
    todo!("AT-TX-01");
}

/// AT-TX-02: Transfer exceeding balance
/// Pass: `InvariantViolated{balance>=0}`, no state change, rejection event
#[test]
#[ignore = "M2"]
fn at_tx_02() {
    todo!("AT-TX-02");
}

/// AT-TX-03: Transaction with failing 3rd op
/// Pass: First two ops not applied
#[test]
#[ignore = "M2"]
fn at_tx_03() {
    todo!("AT-TX-03");
}

/// AT-TX-04: `expect_version` stale
/// Pass: `PreconditionFailed`
#[test]
#[ignore = "M2"]
fn at_tx_04() {
    todo!("AT-TX-04");
}

/// AT-TX-05: 64 concurrent random transfers, 1000 rounds
/// Pass: Total money constant; result equals some serial order (checked by history replay)
#[test]
#[ignore = "M2"]
fn at_tx_05() {
    todo!("AT-TX-05");
}

/// AT-TX-06: Write skew scenario (two txs each read both, write one)
/// Pass: One aborts with `Conflict`
#[test]
#[ignore = "M2"]
fn at_tx_06() {
    todo!("AT-TX-06");
}

/// AT-TX-07: Resubmit committed `tx_id`, same body
/// Pass: Original result returned, no new event
#[test]
#[ignore = "M2"]
fn at_tx_07() {
    todo!("AT-TX-07");
}

/// AT-TX-08: Reuse `tx_id`, different body
/// Pass: `Malformed{tx_id_reuse}`
#[test]
#[ignore = "M2"]
fn at_tx_08() {
    todo!("AT-TX-08");
}

/// AT-TX-09: `dry_run`
/// Pass: `DryRunOk`, no state change, no commit event
#[test]
#[ignore = "M2"]
fn at_tx_09() {
    todo!("AT-TX-09");
}

/// AT-TX-10: Named op that would create money (bad registration)
/// Pass: Registration rejected or invariant fails at commit
#[test]
#[ignore = "M2"]
fn at_tx_10() {
    todo!("AT-TX-10");
}

/// AT-TX-11: Delete then get
/// Pass: Tombstone; `get(at=prev seq)` returns old state
#[test]
#[ignore = "M2"]
fn at_tx_11() {
    todo!("AT-TX-11");
}

/// AT-TX-12: Unique index violation
/// Pass: `SchemaViolation{unique}`
#[test]
#[ignore = "M2"]
fn at_tx_12() {
    todo!("AT-TX-12");
}
