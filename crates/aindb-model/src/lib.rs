//! AINDB data model (SPEC §4): identifiers, values, schemas, objects,
//! events and evidence. Pure data; no I/O and no dependency on other crates.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

/// 128-bit object identifier, engine-assigned or derived from `ns:key`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(pub [u8; 16]);

/// Caller-supplied idempotency key or engine-assigned transaction id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TxId(pub [u8; 16]);

/// Dense, strictly increasing history sequence number.
pub type Seq = u64;

/// Per-object version; 1 on create, +1 per committed change.
pub type Version = u64;

/// 32-byte digest. The algorithm is recorded alongside (see [`HashAlg`]).
pub type Hash = [u8; 32];

/// Schema name and major version, e.g. `account@1`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaId {
    pub name: String,
    pub major: u16,
}

/// Fixed-point decimal: `units * 10^-scale`. Arithmetic is always checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decimal {
    pub units: i128,
    pub scale: u8,
}

impl Decimal {
    /// Checked addition of two decimals with the same scale.
    pub fn checked_add(self, rhs: Decimal) -> Option<Decimal> {
        if self.scale != rhs.scale {
            return None;
        }
        Some(Decimal {
            units: self.units.checked_add(rhs.units)?,
            scale: self.scale,
        })
    }

    /// Checked subtraction of two decimals with the same scale.
    pub fn checked_sub(self, rhs: Decimal) -> Option<Decimal> {
        if self.scale != rhs.scale {
            return None;
        }
        Some(Decimal {
            units: self.units.checked_sub(rhs.units)?,
            scale: self.scale,
        })
    }
}

/// Attribute value (SPEC §4.3). No floating point in v0.1.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    Decimal(Decimal),
    Text(String),
    Bytes(Vec<u8>),
    /// UTC nanoseconds since the Unix epoch.
    Timestamp(i128),
    Ref(ObjectId),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
}

/// Declared type of a schema field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldType {
    Bool,
    I64,
    U64,
    Decimal { scale: u8 },
    Text,
    Bytes,
    Timestamp,
    Ref(SchemaId),
    List(Box<FieldType>),
    Map(Box<FieldType>),
}

/// How a field may appear in explanations and events (SPEC §4.4, A6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sensitivity {
    Public,
    Internal,
    /// Hashed in events, redacted in explanations.
    Secret,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldDef {
    pub name: String,
    pub ty: FieldType,
    pub required: bool,
    pub default: Option<Value>,
    pub sensitivity: Sensitivity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexDef {
    pub name: String,
    pub fields: Vec<String>,
    pub unique: bool,
}

/// Declarative, deterministic invariant. The expression language is
/// defined in M2; until then it is carried as source text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvariantDef {
    pub name: String,
    pub expr: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Schema {
    pub id: SchemaId,
    pub fields: Vec<FieldDef>,
    pub indexes: Vec<IndexDef>,
    pub invariants: Vec<InvariantDef>,
}

/// Current state of an object.
#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    pub id: ObjectId,
    pub schema: SchemaId,
    pub version: Version,
    pub deleted: bool,
    pub fields: BTreeMap<String, Value>,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_arithmetic_is_checked() {
        let max = Decimal {
            units: i128::MAX,
            scale: 2,
        };
        let one = Decimal { units: 1, scale: 2 };
        assert_eq!(max.checked_add(one), None);
        assert_eq!(one.checked_add(Decimal { units: 1, scale: 3 }), None);
        assert_eq!(one.checked_sub(one), Some(Decimal { units: 0, scale: 2 }));
    }
}
