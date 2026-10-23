//! AT-CDK-* acceptance tests. Generated from docs/ACCEPTANCE.md;
//! remove `#[ignore]` when implementing each test in its milestone.

/// AT-CDK-01: Valid CDK capability token for `aindb:acct/*` write
/// Pass: Authorized; event records CDK issuer id and token hash
#[test]
#[ignore = "M5"]
fn at_cdk_01() {
    todo!("AT-CDK-01");
}

/// AT-CDK-02: Expired / revoked token
/// Pass: `Unauthorized`
#[test]
#[ignore = "M5"]
fn at_cdk_02() {
    todo!("AT-CDK-02");
}

/// AT-CDK-03: Token with only Ed25519 or only ML-DSA valid
/// Pass: `Unauthorized` (hybrid requires both)
#[test]
#[ignore = "M5"]
fn at_cdk_03() {
    todo!("AT-CDK-03");
}

/// AT-CDK-04: Checkpoint anchored to CDK audit log
/// Pass: CDK record references AINDB seq + head hash; AT-AUD-07 detects truncation via it
#[test]
#[ignore = "M5"]
fn at_cdk_04() {
    todo!("AT-CDK-04");
}

/// AT-CDK-05: Engine built without `cdk` feature
/// Pass: All non-CDK tests pass
#[test]
#[ignore = "M5"]
fn at_cdk_05() {
    todo!("AT-CDK-05");
}
