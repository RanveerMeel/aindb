//! AT-PROT-* acceptance tests (docs/ACCEPTANCE.md). AT-PROT-03 is M4.

use aindb_acceptance::{request, Gen};
use aindb_model::ObjectId;
use aindb_protocol::{limits, Codec, DecodeError, ErrorCode, Operation, TypedJson};

const VALID: &str = r#"{"protocol":"AINDB-OP-v1","tx_id":"tx-1","principal":{"api_key":"k"},
  "ops":[{"get":{"id":"acct:A-100"}}]}"#;

fn decode(s: &str) -> Result<aindb_protocol::Request, DecodeError> {
    TypedJson.decode_request(s.as_bytes())
}

/// AT-PROT-01: Typed JSON round trip
/// Pass: decode(encode(r)) == r for generated requests
#[test]
fn at_prot_01() {
    let mut g = Gen::new(0x5eed);
    for i in 0..2_000 {
        let r = g.request();
        let bytes = TypedJson.encode_request(&r);
        let back = TypedJson
            .decode_request(&bytes)
            .unwrap_or_else(|e| panic!("case {i}: {e}\n{}", String::from_utf8_lossy(&bytes)));
        assert_eq!(back, r, "case {i}");
        assert_eq!(
            TypedJson.encode_request(&back),
            bytes,
            "case {i}: encoding is stable"
        );
    }
}

/// AT-PROT-02: Reject unknown field / trailing bytes / duplicate key
/// Pass: `Malformed`, nothing executed (decoding fails before the engine runs)
#[test]
fn at_prot_02() {
    assert!(decode(VALID).is_ok());
    let cases = [
        (
            VALID.replace(r#""tx_id""#, r#""extra":1,"tx_id""#),
            "unknown top-level field",
        ),
        (
            VALID.replace(r#"{"id":"acct:A-100"}"#, r#"{"id":"acct:A-100","as_of":1}"#),
            "unknown op field",
        ),
        (
            VALID.replace(r#"{"get""#, r#"{"fetch""#),
            "unknown operation",
        ),
        (format!("{VALID} {{}}"), "trailing bytes"),
        (
            VALID.replace(r#""tx_id":"tx-1""#, r#""tx_id":"tx-1","tx_id":"tx-2""#),
            "duplicate key",
        ),
        (VALID.replace("acct:A-100", "acct A-100"), "bad object id"),
        (
            VALID.replace(r#""ops":["#, r#""ops":[1,"#),
            "op is not an object",
        ),
    ];
    for (bad, what) in cases {
        let err = decode(&bad).expect_err(what);
        assert!(
            matches!(err.code(), ErrorCode::Malformed(_)),
            "{what}: {err}"
        );
    }
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
fn at_prot_04() {
    let err = decode(&VALID.replace("AINDB-OP-v1", "AINDB-OP-v2")).unwrap_err();
    assert_eq!(err, DecodeError::UnsupportedVersion("AINDB-OP-v2".into()));
    assert!(matches!(err.code(), ErrorCode::Malformed(m) if m.contains("unsupported protocol")));
}

/// AT-PROT-05: 257 ops in one request
/// Pass: `LimitExceeded`
#[test]
fn at_prot_05() {
    let get = || Operation::Get {
        id: ObjectId::parse("acct:a").unwrap(),
        at: None,
    };
    let ok = TypedJson.encode_request(&request(vec![get(); limits::MAX_OPS]));
    assert!(TypedJson.decode_request(&ok).is_ok());
    let too_many = TypedJson.encode_request(&request(vec![get(); limits::MAX_OPS + 1]));
    let err = TypedJson.decode_request(&too_many).unwrap_err();
    assert_eq!(err.code(), ErrorCode::LimitExceeded);
}
