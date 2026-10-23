//! Independent-verifier contract (SPEC §8.2). Verifiers see an immutable
//! [`Proposal`] and return a [`Verdict`]; they cannot change the write set.
//! Deterministic invariants are enforced by the engine and are never
//! delegated to a verifier.
#![forbid(unsafe_code)]

use aindb_model::{ObjectId, Principal, Seq, Version, Write};
use aindb_protocol::Request;

/// Everything a verifier may inspect. Borrowed immutably for the call.
#[derive(Debug)]
pub struct Proposal<'a> {
    pub request: &'a Request,
    pub principal: &'a Principal,
    /// Snapshot the transaction executed against; commit is validated
    /// against the same snapshot (SPEC P4).
    pub snapshot_seq: Seq,
    pub reads: &'a [(ObjectId, Version)],
    pub writes: &'a [Write],
    /// Names of invariants that already passed.
    pub invariants_passed: &'a [String],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Accept,
    Reject { reason: String },
    Abstain,
}

/// Whether a verifier's rejection aborts the transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Reject or timeout aborts the transaction.
    Required,
    /// Verdict is recorded as evidence only.
    Advisory,
}

pub trait Verifier: Send + Sync {
    /// Stable id recorded in evidence.
    fn id(&self) -> &str;
    fn verify(&self, proposal: &Proposal<'_>) -> Verdict;
}
