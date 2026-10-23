//! AINDB agent protocol (SPEC §5): typed requests and results, plus the
//! [`Codec`] trait implemented by the typed (JSON) and binary encodings.
//! Both encodings decode into the same [`Request`]; the engine never sees
//! wire bytes (SPEC P3).
#![forbid(unsafe_code)]

use aindb_model::{ObjectId, Schema, SchemaId, Seq, TxId, Value, Version};
use std::collections::BTreeMap;

/// Protocol identifier and domain separator.
pub const PROTOCOL: &str = "AINDB-OP-v1";

/// Default request limits (SPEC §6.4).
pub mod limits {
    pub const MAX_OPS: usize = 256;
    pub const MAX_OBJECT_BYTES: usize = 1 << 20;
    pub const MAX_TX_BYTES: usize = 8 << 20;
    pub const MAX_SCAN: usize = 10_000;
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
        id: Option<ObjectId>,
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
        op: String,
        version: u16,
        inputs: BTreeMap<String, Value>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistorySelector {
    Object(ObjectId),
    Tx(TxId),
    Principal(String),
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    UnsupportedVersion,
    UnknownField(String),
    DuplicateKey(String),
    TrailingBytes,
    NonCanonical,
    LimitExceeded,
    Invalid(String),
}

/// A wire encoding of the protocol. Implementations MUST be strict:
/// reject unknown fields, duplicate keys, trailing bytes and oversize input.
pub trait Codec {
    fn decode_request(&self, bytes: &[u8]) -> Result<Request, DecodeError>;
    fn encode_request(&self, req: &Request) -> Vec<u8>;
    fn encode_result(&self, res: &TxResult) -> Vec<u8>;
}

/// Shape checks independent of encoding (op count, etc.).
pub fn check_limits(req: &Request) -> Result<(), DecodeError> {
    if req.ops.is_empty() || req.ops.len() > limits::MAX_OPS {
        return Err(DecodeError::LimitExceeded);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(n: usize) -> Request {
        Request {
            tx_id: TxId([0; 16]),
            principal: PrincipalClaim::ApiKey("k".into()),
            ops: vec![
                Operation::Get {
                    id: ObjectId([1; 16]),
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
        assert_eq!(check_limits(&req(0)), Err(DecodeError::LimitExceeded));
        assert_eq!(
            check_limits(&req(limits::MAX_OPS + 1)),
            Err(DecodeError::LimitExceeded)
        );
    }
}
