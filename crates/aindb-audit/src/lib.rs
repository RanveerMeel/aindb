//! History integrity and explanations (SPEC §7).
//!
//! * [`ChainVerifier`] checks sequence density and hash links.
//! * [`Checkpoint`] is the signed statement of the history head.
//! * [`Explainer`] renders events for humans, from records only.
//!
//! Hashing (SHA-256) and hybrid signatures are wired in M3 (ADR-0003,
//! ADR-0005); this crate defines the shapes and the pure checks.
#![forbid(unsafe_code)]

use aindb_model::{Event, Hash, HashAlg, Seq, SigAlg};

pub const EVENT_DOMAIN: &[u8] = b"AINDB-EVENT-v1";
pub const CHECKPOINT_DOMAIN: &[u8] = b"AINDB-CKPT-v1";
pub const ANCHOR_DOMAIN: &[u8] = b"AINDB-ANCHOR-v1";

/// Hash of the (empty) history before seq 1.
pub const GENESIS: Hash = [0; 32];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub node_id: [u8; 16],
    pub seq: Seq,
    pub head_hash: Hash,
    pub hash_alg: HashAlg,
    pub sig_alg: SigAlg,
    pub ed25519_sig: Vec<u8>,
    pub ml_dsa_sig: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum IntegrityError {
    /// First event whose `seq` is not previous + 1.
    Gap { expected: Seq, found: Seq },
    /// First event whose `prev_hash` does not match its predecessor.
    ChainBreak { seq: Seq },
    /// First event whose stored hash does not match its recomputed hash.
    BadHash { seq: Seq },
}

/// Computes an event's hash over `EVENT_DOMAIN ‖ seq ‖ prev_hash ‖ body`.
pub trait EventHasher {
    fn hash(&self, event: &Event) -> Hash;
}

/// Walks a history in order and reports the first integrity failure.
pub struct ChainVerifier<'h, H: EventHasher> {
    hasher: &'h H,
    expected_seq: Seq,
    prev: Hash,
}

impl<'h, H: EventHasher> ChainVerifier<'h, H> {
    pub fn new(hasher: &'h H) -> Self {
        Self {
            hasher,
            expected_seq: 1,
            prev: GENESIS,
        }
    }

    pub fn push(&mut self, e: &Event) -> Result<(), IntegrityError> {
        if e.seq != self.expected_seq {
            return Err(IntegrityError::Gap {
                expected: self.expected_seq,
                found: e.seq,
            });
        }
        if e.prev_hash != self.prev {
            return Err(IntegrityError::ChainBreak { seq: e.seq });
        }
        if self.hasher.hash(e) != e.hash {
            return Err(IntegrityError::BadHash { seq: e.seq });
        }
        self.prev = e.hash;
        self.expected_seq += 1;
        Ok(())
    }

    /// Head hash after the last accepted event.
    pub fn head(&self) -> Hash {
        self.prev
    }
}

/// Renders the six audit questions (SPEC §7.3) for an event.
pub trait Explainer {
    fn explain(&self, event: &Event) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;
    use aindb_model::*;

    /// Test-only hasher: not cryptographic, just binds seq and prev_hash.
    struct ToyHasher;
    impl EventHasher for ToyHasher {
        fn hash(&self, e: &Event) -> Hash {
            let mut h = e.prev_hash;
            h[0] ^= e.seq as u8;
            h[31] = h[31].wrapping_add(1);
            h
        }
    }

    fn chain(n: u64) -> Vec<Event> {
        let mut prev = GENESIS;
        (1..=n)
            .map(|seq| {
                let mut e = Event {
                    seq,
                    prev_hash: prev,
                    hash: [0; 32],
                    hash_alg: HashAlg::Sha256,
                    time_ns: 0,
                    kind: EventKind::TxCommitted,
                    tx_id: TxId::parse("tx-0").unwrap(),
                    principal: Principal {
                        id: "agent".into(),
                        delegation: vec![],
                        auth_method: AuthMethod::ApiKey,
                    },
                    claimed_reason: String::new(),
                    ops: vec![],
                    writes: vec![],
                    evidence: Evidence {
                        authz_decision: "allow".into(),
                        policy_ref: "v1#0".into(),
                        checks: vec![],
                        verifiers: vec![],
                    },
                };
                e.hash = ToyHasher.hash(&e);
                prev = e.hash;
                e
            })
            .collect()
    }

    fn verify(events: &[Event]) -> Result<(), IntegrityError> {
        let mut v = ChainVerifier::new(&ToyHasher);
        events.iter().try_for_each(|e| v.push(e))
    }

    #[test]
    fn detects_gap_tamper_and_chain_break() {
        let good = chain(4);
        assert_eq!(verify(&good), Ok(()));

        let mut gap = good.clone();
        gap.remove(1);
        assert_eq!(
            verify(&gap),
            Err(IntegrityError::Gap {
                expected: 2,
                found: 3
            })
        );

        let mut tampered = good.clone();
        tampered[2].hash[5] ^= 1;
        assert_eq!(verify(&tampered), Err(IntegrityError::BadHash { seq: 3 }));

        let mut broken = good;
        broken[2].prev_hash[9] ^= 1;
        assert_eq!(verify(&broken), Err(IntegrityError::ChainBreak { seq: 3 }));
    }
}
