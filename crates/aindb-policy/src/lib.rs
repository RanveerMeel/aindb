//! Authentication and authorization (SPEC §8.1). The engine calls an
//! [`Authorizer`] for every request; standalone mode uses [`StaticPolicy`],
//! CDK mode uses `aindb_cdk::CdkAuthorizer`.
#![forbid(unsafe_code)]

use aindb_model::{Principal, SchemaId};
use aindb_protocol::PrincipalClaim;

/// Operation class used for authorization decisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpClass {
    Read,
    Write,
    Delete,
    /// Named (composite) operation.
    Execute,
    /// Schema definition and named-op registration.
    Schema,
}

/// What the authorizer is asked about: one operation, without values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpSummary {
    pub class: OpClass,
    /// Namespace of the target object(s), e.g. `acct`.
    pub namespace: String,
    pub schema: Option<SchemaId>,
    /// `name@version` for named operations.
    pub named_op: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// `policy_ref` identifies the rule and policy version, kept as evidence.
    Allow {
        policy_ref: String,
    },
    Deny {
        reason: String,
    },
    /// A human approval is required (resolved outside AINDB, e.g. by CDK).
    NeedsApproval {
        reason: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum AuthError {
    UnknownCredential,
    BadSignature,
    Expired,
    Revoked,
}

pub trait Authorizer: Send + Sync {
    fn authenticate(&self, claim: &PrincipalClaim) -> Result<Principal, AuthError>;
    fn authorize(&self, principal: &Principal, op: &OpSummary) -> Decision;
}

/// One standalone policy rule. `principal` and `namespace` accept `*`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub principal: String,
    pub class: OpClass,
    pub namespace: String,
    pub allow: bool,
}

/// Standalone, deny-by-default policy. File format and key handling land
/// in M2; this is the in-memory form.
#[derive(Clone, Debug, Default)]
pub struct StaticPolicy {
    pub version: String,
    pub api_keys: Vec<(String, String)>, // (key, principal id)
    pub rules: Vec<Rule>,
}

impl Authorizer for StaticPolicy {
    fn authenticate(&self, claim: &PrincipalClaim) -> Result<Principal, AuthError> {
        match claim {
            PrincipalClaim::ApiKey(k) => self
                .api_keys
                .iter()
                .find(|(key, _)| key == k)
                .map(|(_, id)| Principal {
                    id: id.clone(),
                    delegation: vec![],
                    auth_method: aindb_model::AuthMethod::ApiKey,
                })
                .ok_or(AuthError::UnknownCredential),
            _ => Err(AuthError::UnknownCredential),
        }
    }

    fn authorize(&self, principal: &Principal, op: &OpSummary) -> Decision {
        let matches = |pat: &str, v: &str| pat == "*" || pat == v;
        // First matching rule wins; no match denies.
        for (i, r) in self.rules.iter().enumerate() {
            if r.class == op.class
                && matches(&r.principal, &principal.id)
                && matches(&r.namespace, &op.namespace)
            {
                let policy_ref = format!("{}#{}", self.version, i);
                return if r.allow {
                    Decision::Allow { policy_ref }
                } else {
                    Decision::Deny { reason: policy_ref }
                };
            }
        }
        Decision::Deny {
            reason: "no matching rule".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_by_default_and_namespace_scoping() {
        let p = StaticPolicy {
            version: "v1".into(),
            api_keys: vec![("k1".into(), "agent-17".into())],
            rules: vec![Rule {
                principal: "agent-17".into(),
                class: OpClass::Read,
                namespace: "acct".into(),
                allow: true,
            }],
        };
        let who = p
            .authenticate(&PrincipalClaim::ApiKey("k1".into()))
            .unwrap();
        let op = |class, ns: &str| OpSummary {
            class,
            namespace: ns.into(),
            schema: None,
            named_op: None,
        };
        assert!(matches!(
            p.authorize(&who, &op(OpClass::Read, "acct")),
            Decision::Allow { .. }
        ));
        assert!(matches!(
            p.authorize(&who, &op(OpClass::Write, "acct")),
            Decision::Deny { .. }
        ));
        assert!(matches!(
            p.authorize(&who, &op(OpClass::Read, "hr")),
            Decision::Deny { .. }
        ));
        assert_eq!(
            p.authenticate(&PrincipalClaim::ApiKey("nope".into())),
            Err(AuthError::UnknownCredential)
        );
    }
}
