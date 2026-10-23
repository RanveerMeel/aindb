//! CDK adapter (docs/CDK_INTEGRATION.md). Uses only CDK's published wire
//! formats; the `cdk-link` dependency is added in M5.
//!
//! * [`CdkAuthorizer`] — verifies hybrid-signed CDK capability tokens and
//!   maps CDK permissions to AINDB op classes.
//! * [`anchor_message`] — the record AINDB submits to CDK's audit log for
//!   each checkpoint.
#![forbid(unsafe_code)]

use aindb_audit::{Checkpoint, ANCHOR_DOMAIN};
use aindb_model::Principal;
use aindb_policy::{AuthError, Authorizer, Decision, OpClass, OpSummary};
use aindb_protocol::PrincipalClaim;

/// CDK object-name prefix for AINDB namespaces: `aindb:<db>/<namespace>`.
pub const OBJECT_PREFIX: &str = "aindb:";

/// CDK `Permission` values relevant to AINDB (mirrors CDK's capability.rs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CdkPermission {
    Read,
    Write,
    Execute,
    Delete,
    RequiresApproval,
}

/// Permission a CDK token must carry for an AINDB op class.
pub fn required_permission(class: OpClass) -> CdkPermission {
    match class {
        OpClass::Read => CdkPermission::Read,
        OpClass::Write | OpClass::Schema => CdkPermission::Write,
        OpClass::Delete => CdkPermission::Delete,
        OpClass::Execute => CdkPermission::Execute,
    }
}

/// Pinned CDK issuer / registry public keys (Ed25519 ‖ ML-DSA-65).
pub struct TrustAnchors {
    pub registry_ed25519: Vec<u8>,
    pub registry_ml_dsa: Vec<u8>,
}

pub struct CdkAuthorizer {
    pub db_name: String,
    pub anchors: TrustAnchors,
}

impl Authorizer for CdkAuthorizer {
    fn authenticate(&self, claim: &PrincipalClaim) -> Result<Principal, AuthError> {
        match claim {
            // M5: parse token, verify BOTH signatures, check expiry and
            // revocation, build Principal with AuthMethod::CdkCapability.
            PrincipalClaim::Cdk(_) => Err(AuthError::BadSignature),
            _ => Err(AuthError::UnknownCredential),
        }
    }

    fn authorize(&self, _principal: &Principal, _op: &OpSummary) -> Decision {
        Decision::Deny {
            reason: "CDK authorization not implemented (M5)".into(),
        }
    }
}

/// Bytes recorded in the CDK audit log to witness an AINDB checkpoint:
/// `AINDB-ANCHOR-v1 ‖ node_id ‖ seq (BE) ‖ head_hash`.
pub fn anchor_message(cp: &Checkpoint) -> Vec<u8> {
    let mut m = Vec::with_capacity(ANCHOR_DOMAIN.len() + 16 + 8 + 32);
    m.extend_from_slice(ANCHOR_DOMAIN);
    m.extend_from_slice(&cp.node_id);
    m.extend_from_slice(&cp.seq.to_be_bytes());
    m.extend_from_slice(&cp.head_hash);
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_changes_need_write() {
        assert_eq!(required_permission(OpClass::Schema), CdkPermission::Write);
        assert_eq!(required_permission(OpClass::Delete), CdkPermission::Delete);
    }
}
