//! AINDB agent protocol (SPEC §5): typed requests and results, plus the
//! [`Codec`] trait implemented by the typed (JSON) and binary encodings.
//! Both encodings decode into the same [`Request`]; the engine never sees
//! wire bytes (SPEC P3).
#![forbid(unsafe_code)]

pub mod json;
mod typed;

pub use typed::TypedJson;

use aindb_model::{ObjectId, Schema, SchemaId, Seq, TxId, Value, Version};
use std::collections::BTreeMap;
use std::fmt;

/// Protocol identifier and domain separator.
pub const PROTOCOL: &str = "AINDB-OP-v1";

/// Default request limits (SPEC §6.4).
pub mod limits {
    /// Operations per request.
    pub const MAX_OPS: usize = 256;
    /// Canonical typed encoding of one object state or patch.
    pub const MAX_OBJECT_BYTES: usize = 1 << 20;
    /// Encoded request.
    pub const MAX_TX_BYTES: usize = 8 << 20;
    /// Entries returned by one scan or history read.
    pub const MAX_SCAN: u32 = 10_000;
    /// Preconditions per request.
    pub const MAX_PRECONDITIONS: usize = 256;
}

/// Credential presented by the agent. Authentication turns it into a
/// [`aindb_model::Principal`]; nothing in the claim is trusted before that.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrincipalClaim {
    ApiKey(String),
    Ed25519 {
        key_id: String,
        signature: Vec<u8>,
    },
    /// Serialized CDK capability token (verified by `aindb-cdk`).
    Cdk(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    Create {
        schema: SchemaId,
        id: ObjectId,
        state: BTreeMap<String, Value>,
    },
    Update {
        id: ObjectId,
        expect_version: Option<Version>,
        patch: BTreeMap<String, Value>,
    },
    Delete {
        id: ObjectId,
        expect_version: Option<Version>,
    },
    Get {
        id: ObjectId,
        at: Option<Seq>,
    },
    Scan {
        index: String,
        start: Vec<Value>,
        end: Vec<Value>,
        limit: u32,
    },
    History {
        selector: HistorySelector,
        from: Seq,
        limit: u32,
    },
    DefineSchema(Schema),
    /// Schema-registered composite operation, e.g. `transfer@1`.
    Named {
        op: SchemaId,
        inputs: BTreeMap<String, Value>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistorySelector {
    Object(ObjectId),
    Tx(TxId),
    Principal(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Precondition {
    Exists(ObjectId),
    NotExists(ObjectId),
    VersionIs(ObjectId, Version),
}

/// Agent-declared purpose. Recorded as *claimed* in history.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Intent {
    pub reason: String,
    pub request_ref: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub dry_run: bool,
    pub want_explanation: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub tx_id: TxId,
    pub principal: PrincipalClaim,
    pub ops: Vec<Operation>,
    pub preconditions: Vec<Precondition>,
    pub intent: Intent,
    pub options: Options,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Committed,
    Rejected,
    Aborted,
    DryRunOk,
}

/// Typed error codes so agents can react without parsing prose (SPEC §5.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    Malformed(String),
    Unauthorized,
    NeedsApproval,
    SchemaViolation { field_path: String },
    PreconditionFailed,
    InvariantViolated { name: String },
    Conflict,
    VerifierRejected { verifier: String },
    LimitExceeded,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub seq: Seq,
    pub event_hash: aindb_model::Hash,
    pub checkpoint_seq: Option<Seq>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TxResult {
    pub tx_id: TxId,
    pub status: Status,
    pub reads: Vec<(ObjectId, Version)>,
    pub writes: Vec<(ObjectId, Version)>,
    pub values: Vec<Value>,
    pub error: Option<ErrorCode>,
    pub receipt: Option<Receipt>,
    pub explanation: Option<String>,
}

/// Why a request could not be decoded. Every variant means nothing was
/// executed. Paths use `$.ops.0.create.state.balance` notation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    UnsupportedVersion(String),
    UnknownField(String),
    DuplicateKey(String),
    TrailingBytes,
    NonCanonical(String),
    LimitExceeded(String),
    Invalid(String),
}

impl DecodeError {
    /// Error code returned to the agent (SPEC §5.2).
    pub fn code(&self) -> ErrorCode {
        match self {
            DecodeError::LimitExceeded(_) => ErrorCode::LimitExceeded,
            other => ErrorCode::Malformed(other.to_string()),
        }
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::UnsupportedVersion(v) => {
                write!(f, "unsupported protocol {v:?}, expected {PROTOCOL}")
            }
            DecodeError::UnknownField(p) => write!(f, "unknown field {p}"),
            DecodeError::DuplicateKey(p) => write!(f, "duplicate key {p}"),
            DecodeError::TrailingBytes => f.write_str("trailing bytes after request"),
            DecodeError::NonCanonical(m) => write!(f, "non-canonical encoding: {m}"),
            DecodeError::LimitExceeded(m) => write!(f, "limit exceeded: {m}"),
            DecodeError::Invalid(m) => write!(f, "invalid request: {m}"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// A wire encoding of the protocol. Implementations MUST be strict:
/// reject unknown fields, duplicate keys, trailing bytes and oversize input.
pub trait Codec {
    fn decode_request(&self, bytes: &[u8]) -> Result<Request, DecodeError>;
    fn encode_request(&self, req: &Request) -> Vec<u8>;
    fn encode_result(&self, res: &TxResult) -> Vec<u8>;
}

/// Shape checks independent of encoding. Codecs call this after decoding.
pub fn check_limits(req: &Request) -> Result<(), DecodeError> {
    if req.ops.is_empty() || req.ops.len() > limits::MAX_OPS {
        return Err(DecodeError::LimitExceeded(format!(
            "{} operations, allowed 1..={}",
            req.ops.len(),
            limits::MAX_OPS
        )));
    }
    if req.preconditions.len() > limits::MAX_PRECONDITIONS {
        return Err(DecodeError::LimitExceeded(format!(
            "more than {} preconditions",
            limits::MAX_PRECONDITIONS
        )));
    }
    for op in &req.ops {
        if let Operation::Scan { limit, .. } | Operation::History { limit, .. } = op {
            if *limit == 0 || *limit > limits::MAX_SCAN {
                return Err(DecodeError::LimitExceeded(format!(
                    "read limit {limit}, allowed 1..={}",
                    limits::MAX_SCAN
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(n: usize) -> Request {
        Request {
            tx_id: TxId::parse("tx-1").unwrap(),
            principal: PrincipalClaim::ApiKey("k".into()),
            ops: vec![
                Operation::Get {
                    id: ObjectId::parse("acct:a").unwrap(),
                    at: None
                };
                n
            ],
            preconditions: vec![],
            intent: Intent::default(),
            options: Options::default(),
        }
    }

    #[test]
    fn op_count_limits() {
        assert!(check_limits(&req(1)).is_ok());
        assert!(matches!(
            check_limits(&req(0)),
            Err(DecodeError::LimitExceeded(_))
        ));
        assert!(matches!(
            check_limits(&req(limits::MAX_OPS + 1)),
            Err(DecodeError::LimitExceeded(_))
        ));
    }
}
