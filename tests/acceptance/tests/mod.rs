//! AT-MOD-* acceptance tests (docs/ACCEPTANCE.md, milestone M1).

use aindb_acceptance::{account_schema, dec, fields};
use aindb_model::{Decimal, ModelError, Value};
use aindb_protocol::{limits, Codec, ErrorCode, TypedJson};

/// AT-MOD-01: Create object with valid schema
/// Pass: all required fields present (version 1 is assigned by the engine, M2)
#[test]
fn at_mod_01() {
    let s = account_schema();
    assert_eq!(s.check(), Ok(()));
    let state = s
        .validate_state(&fields(&[
            ("owner", Value::Text("asha".into())),
            ("balance", dec("10000.00")),
        ]))
        .unwrap();
    assert_eq!(state.len(), 3, "default currency filled in");
    assert_eq!(state["currency"], Value::Text("INR".into()));
}

/// AT-MOD-02: Missing required field
/// Pass: `SchemaViolation{field_path}`
#[test]
fn at_mod_02() {
    let err = account_schema()
        .validate_state(&fields(&[("owner", Value::Text("asha".into()))]))
        .unwrap_err();
    assert!(matches!(err, ModelError::SchemaViolation { ref path, .. } if path == "balance"));
}

/// AT-MOD-03: Wrong field type
/// Pass: `SchemaViolation{field_path}`
#[test]
fn at_mod_03() {
    let s = account_schema();
    for bad in [Value::I64(10), Value::Text("10.00".into()), dec("10.0")] {
        let err = s
            .validate_state(&fields(&[
                ("owner", Value::Text("asha".into())),
                ("balance", bad),
            ]))
            .unwrap_err();
        assert!(matches!(err, ModelError::SchemaViolation { ref path, .. } if path == "balance"));
    }
}

/// AT-MOD-04: Decimal overflow in arithmetic
/// Pass: checked arithmetic reports overflow (engine abort is AT-TX-03, M2)
#[test]
fn at_mod_04() {
    let max = Decimal {
        units: i128::MAX,
        scale: 2,
    };
    let cent = Decimal::parse("0.01").unwrap();
    assert_eq!(max.checked_add(cent), Err(ModelError::Arithmetic));
    let min = Decimal {
        units: i128::MIN,
        scale: 2,
    };
    assert_eq!(min.checked_sub(cent), Err(ModelError::Arithmetic));
    assert!(
        Decimal::parse(&"9".repeat(39)).is_err(),
        "parse rejects out-of-range"
    );
}

/// AT-MOD-05: Object larger than limit
/// Pass: `LimitExceeded`
#[test]
fn at_mod_05() {
    let big = "x".repeat(limits::MAX_OBJECT_BYTES);
    let req = format!(
        r#"{{"protocol":"AINDB-OP-v1","tx_id":"tx-1","principal":{{"api_key":"k"}},
            "ops":[{{"create":{{"schema":"account@1","id":"acct:a","state":{{"owner":"{big}"}}}}}}]}}"#
    );
    let err = TypedJson.decode_request(req.as_bytes()).unwrap_err();
    assert_eq!(err.code(), ErrorCode::LimitExceeded);

    let small = req.replace(&big, "asha");
    assert!(TypedJson.decode_request(small.as_bytes()).is_ok());
}
