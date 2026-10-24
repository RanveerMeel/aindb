//! History records and evidence (SPEC §7.1).

use crate::{Hash, ObjectId, Seq, TxId, Value, Version};

/// Hash and signature algorithm identifiers, recorded in every event and
/// checkpoint for crypto agility (SPEC P7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum HashAlg {
    Sha256 = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SigAlg {
    /// Ed25519 and ML-DSA-65; valid only if both verify.
    HybridEd25519MlDsa65 = 1,
}

/// Authenticated identity an operation runs as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Principal {
    pub id: String,
    /// Delegation chain, outermost first (e.g. operator → agent).
    pub delegation: Vec<String>,
    pub auth_method: AuthMethod,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthMethod {
    ApiKey,
    Ed25519Key,
    /// CDK capability token; issuer id and token hash are kept as evidence.
    CdkCapability {
        issuer: [u8; 16],
        token_hash: Hash,
    },
}

/// One field-level change recorded in an event.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldChange {
    pub field: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Write {
    pub object: ObjectId,
    pub old_version: Option<Version>,
    pub new_version: Version,
    pub changes: Vec<FieldChange>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckResult {
    Passed,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerdictRecord {
    Accept,
    Reject(String),
    Abstain,
    TimedOut,
}

/// Why an event was allowed or rejected (SPEC §7.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    pub authz_decision: String,
    pub policy_ref: String,
    pub checks: Vec<(String, CheckResult)>,
    pub verifiers: Vec<(String, VerdictRecord)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventKind {
    TxCommitted,
    TxRejected,
    SchemaChanged,
    System(String),
}

/// Immutable history record (SPEC §7.1).
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub seq: Seq,
    pub prev_hash: Hash,
    pub hash: Hash,
    pub hash_alg: HashAlg,
    /// Engine clock, informational only; not used for ordering.
    pub time_ns: i128,
    pub kind: EventKind,
    pub tx_id: TxId,
    pub principal: Principal,
    /// What the agent said it was doing. Recorded, never trusted.
    pub claimed_reason: String,
    /// Canonical binary form of the submitted operations.
    pub ops: Vec<u8>,
    pub writes: Vec<Write>,
    pub evidence: Evidence,
}
