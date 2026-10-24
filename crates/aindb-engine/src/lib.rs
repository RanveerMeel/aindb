//! Transaction engine (SPEC §6). The only crate that writes to storage.
//!
//! Lifecycle of a request:
//! decode → authenticate → authorize → snapshot → preconditions → apply to
//! private write set → schema checks → invariants → verifiers →
//! commit-validate read set → one atomic batch {state, indexes, event,
//! chain head} → result + receipt.
//!
//! M0 implements only the stages that need no storage layout: limits and
//! authentication. Everything after returns `Internal` until M2.
#![forbid(unsafe_code)]

use aindb_policy::Authorizer;
use aindb_protocol::{check_limits, ErrorCode, Request, Status, TxResult};
use aindb_storage::Backend;
use aindb_verify::{Mode, Verifier};

pub struct Config {
    /// Sign a checkpoint every N events (ADR-0005).
    pub checkpoint_interval: u64,
    pub verifier_timeout_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            checkpoint_interval: 1024,
            verifier_timeout_ms: 5_000,
        }
    }
}

pub struct Engine {
    #[allow(dead_code)] // used from M2
    backend: Box<dyn Backend>,
    authorizer: Box<dyn Authorizer>,
    verifiers: Vec<(Box<dyn Verifier>, Mode)>,
    #[allow(dead_code)]
    config: Config,
}

impl Engine {
    pub fn new(backend: Box<dyn Backend>, authorizer: Box<dyn Authorizer>, config: Config) -> Self {
        Self {
            backend,
            authorizer,
            verifiers: Vec::new(),
            config,
        }
    }

    pub fn add_verifier(&mut self, v: Box<dyn Verifier>, mode: Mode) {
        self.verifiers.push((v, mode));
    }

    pub fn submit(&self, req: &Request) -> TxResult {
        let fail = |status, code| TxResult {
            tx_id: req.tx_id.clone(),
            status,
            reads: vec![],
            writes: vec![],
            values: vec![],
            error: Some(code),
            receipt: None,
            explanation: None,
        };
        if check_limits(req).is_err() {
            return fail(Status::Rejected, ErrorCode::LimitExceeded);
        }
        if self.authorizer.authenticate(&req.principal).is_err() {
            return fail(Status::Rejected, ErrorCode::Unauthorized);
        }
        // M2: authorize each op, snapshot, execute, invariants, verifiers,
        // commit-validate, atomic batch, rejection events.
        fail(Status::Aborted, ErrorCode::Internal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aindb_model::{ObjectId, TxId};
    use aindb_policy::StaticPolicy;
    use aindb_protocol::{Intent, Operation, Options, PrincipalClaim};
    use aindb_storage::MemoryBackend;

    #[test]
    fn unknown_credential_is_rejected() {
        let engine = Engine::new(
            Box::new(MemoryBackend::default()),
            Box::new(StaticPolicy::default()),
            Config::default(),
        );
        let req = Request {
            tx_id: TxId::parse("tx-7").unwrap(),
            principal: PrincipalClaim::ApiKey("nope".into()),
            ops: vec![Operation::Get {
                id: ObjectId::parse("acct:a").unwrap(),
                at: None,
            }],
            preconditions: vec![],
            intent: Intent::default(),
            options: Options::default(),
        };
        let res = engine.submit(&req);
        assert_eq!(res.status, Status::Rejected);
        assert_eq!(res.error, Some(ErrorCode::Unauthorized));
    }
}
