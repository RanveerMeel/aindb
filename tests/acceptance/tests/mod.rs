//! AT-MOD-* acceptance tests. Generated from docs/ACCEPTANCE.md;
//! remove `#[ignore]` when implementing each test in its milestone.

/// AT-MOD-01: Create object with valid schema
/// Pass: Version 1, all required fields present
#[test]
#[ignore = "M1"]
fn at_mod_01() {
    todo!("AT-MOD-01");
}

/// AT-MOD-02: Missing required field
/// Pass: `SchemaViolation{field_path}`
#[test]
#[ignore = "M1"]
fn at_mod_02() {
    todo!("AT-MOD-02");
}

/// AT-MOD-03: Wrong field type
/// Pass: `SchemaViolation{field_path}`
#[test]
#[ignore = "M1"]
fn at_mod_03() {
    todo!("AT-MOD-03");
}

/// AT-MOD-04: Decimal overflow in arithmetic
/// Pass: Transaction aborted, no state change
#[test]
#[ignore = "M1"]
fn at_mod_04() {
    todo!("AT-MOD-04");
}

/// AT-MOD-05: Object larger than limit
/// Pass: `LimitExceeded`
#[test]
#[ignore = "M1"]
fn at_mod_05() {
    todo!("AT-MOD-05");
}
