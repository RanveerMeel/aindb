//! AT-AUTH-* acceptance tests. Generated from docs/ACCEPTANCE.md;
//! remove `#[ignore]` when implementing each test in its milestone.

/// AT-AUTH-01: Unknown credential
/// Pass: `Unauthorized`, rejection event with no write set
#[test]
#[ignore = "M2"]
fn at_auth_01() {
    todo!("AT-AUTH-01");
}

/// AT-AUTH-02: Agent allowed `get` only attempts `update`
/// Pass: `Unauthorized`
#[test]
#[ignore = "M2"]
fn at_auth_02() {
    todo!("AT-AUTH-02");
}

/// AT-AUTH-03: Agent restricted to namespace `acct:` writes `hr:`
/// Pass: `Unauthorized`
#[test]
#[ignore = "M2"]
fn at_auth_03() {
    todo!("AT-AUTH-03");
}

/// AT-AUTH-04: Principal field in `intent` spoofed
/// Pass: Event records authenticated principal, not claimed one
#[test]
#[ignore = "M2"]
fn at_auth_04() {
    todo!("AT-AUTH-04");
}

/// AT-AUTH-05: Policy file changed
/// Pass: Event records new policy version; old events keep old version
#[test]
#[ignore = "M2"]
fn at_auth_05() {
    todo!("AT-AUTH-05");
}
