//! Identifiers (SPEC §4.2). All are validated on construction, so a value
//! of these types is always well-formed.

use crate::ModelError;
use std::fmt;

const MAX_OBJECT_ID: usize = 255;
const MAX_NS: usize = 32;
const MAX_TX_ID: usize = 64;
const MAX_NAME: usize = 64;

fn is_ns_char(c: u8) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-'
}

fn is_key_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.')
}

/// Lowercase identifier: `[a-z][a-z0-9_]*`, at most 64 bytes. Used for
/// field, index, schema and named-operation names.
pub fn check_name(s: &str) -> Result<(), ModelError> {
    let b = s.as_bytes();
    let ok = !b.is_empty()
        && b.len() <= MAX_NAME
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|&c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_');
    if ok {
        Ok(())
    } else {
        Err(ModelError::InvalidId(format!("name {s:?}")))
    }
}

/// Object identifier `ns:key`. The namespace is the unit of authorization
/// (SPEC §8.1); the key is chosen by the creating agent.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId {
    ns: String,
    key: String,
}

impl ObjectId {
    /// `ns`: `[a-z0-9_-]{1,32}`; `key`: `[A-Za-z0-9._-]+`; whole id ≤ 255 bytes.
    pub fn new(ns: &str, key: &str) -> Result<Self, ModelError> {
        let bad = || ModelError::InvalidId(format!("object id {ns:?}:{key:?}"));
        if ns.is_empty() || ns.len() > MAX_NS || !ns.bytes().all(is_ns_char) {
            return Err(bad());
        }
        if key.is_empty()
            || ns.len() + 1 + key.len() > MAX_OBJECT_ID
            || !key.bytes().all(is_key_char)
        {
            return Err(bad());
        }
        Ok(Self {
            ns: ns.to_owned(),
            key: key.to_owned(),
        })
    }

    /// Parses `ns:key`.
    pub fn parse(s: &str) -> Result<Self, ModelError> {
        let (ns, key) = s
            .split_once(':')
            .ok_or_else(|| ModelError::InvalidId(format!("object id {s:?}")))?;
        Self::new(ns, key)
    }

    pub fn ns(&self) -> &str {
        &self.ns
    }

    pub fn key(&self) -> &str {
        &self.key
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.ns, self.key)
    }
}

/// Transaction id, supplied by the agent as an idempotency key:
/// `[A-Za-z0-9._:-]{1,64}`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TxId(String);

impl TxId {
    pub fn parse(s: &str) -> Result<Self, ModelError> {
        let ok =
            !s.is_empty() && s.len() <= MAX_TX_ID && s.bytes().all(|c| is_key_char(c) || c == b':');
        if ok {
            Ok(Self(s.to_owned()))
        } else {
            Err(ModelError::InvalidId(format!("tx id {s:?}")))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TxId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Versioned name `name@major`, used for schemas (`account@1`) and named
/// operations (`transfer@1`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaId {
    pub name: String,
    pub major: u16,
}

impl SchemaId {
    pub fn new(name: &str, major: u16) -> Result<Self, ModelError> {
        check_name(name)?;
        if major == 0 {
            return Err(ModelError::InvalidId(format!(
                "{name}@0: major starts at 1"
            )));
        }
        Ok(Self {
            name: name.to_owned(),
            major,
        })
    }

    /// Parses `name@major`; the major version has no leading zeros.
    pub fn parse(s: &str) -> Result<Self, ModelError> {
        let bad = || ModelError::InvalidId(format!("versioned name {s:?}"));
        let (name, major) = s.split_once('@').ok_or_else(bad)?;
        if major.is_empty() || major.starts_with('0') || !major.bytes().all(|c| c.is_ascii_digit())
        {
            return Err(bad());
        }
        Self::new(name, major.parse().map_err(|_| bad())?)
    }
}

impl fmt::Display for SchemaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.name, self.major)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_ids() {
        let id = ObjectId::parse("acct:A-100").unwrap();
        assert_eq!((id.ns(), id.key()), ("acct", "A-100"));
        assert_eq!(id.to_string(), "acct:A-100");
        for bad in ["acct", ":x", "acct:", "Acct:x", "acct:a b", "acct:a:b"] {
            assert!(ObjectId::parse(bad).is_err(), "{bad}");
        }
        assert!(ObjectId::new("a", &"k".repeat(253)).is_ok());
        assert!(ObjectId::new("a", &"k".repeat(254)).is_err());
    }

    #[test]
    fn schema_ids_and_tx_ids() {
        assert_eq!(
            SchemaId::parse("account@1").unwrap().to_string(),
            "account@1"
        );
        for bad in [
            "account",
            "account@",
            "account@0",
            "account@01",
            "Account@1",
            "a@x",
        ] {
            assert!(SchemaId::parse(bad).is_err(), "{bad}");
        }
        assert!(TxId::parse("tx-1042").is_ok());
        assert!(TxId::parse("").is_err());
        assert!(TxId::parse("tx 1").is_err());
    }
}
