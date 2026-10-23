//! AINDB data model (SPEC §4): identifiers, values, schemas, objects,
//! events and evidence. Pure data and validation; no I/O and no dependency
//! on other workspace crates.
#![forbid(unsafe_code)]

mod event;
mod id;
mod schema;
mod value;

pub use event::*;
pub use id::*;
pub use schema::*;
pub use value::*;

/// Per-object version; 1 on create, +1 per committed change.
pub type Version = u64;

/// Dense, strictly increasing history sequence number.
pub type Seq = u64;

/// 32-byte digest. The algorithm is recorded alongside (see [`HashAlg`]).
pub type Hash = [u8; 32];

/// Validation failure in the data model. `path` locates the offending
/// field, e.g. `balance` or `tags[2]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelError {
    /// An identifier (object id, schema id, tx id, field name) is malformed.
    InvalidId(String),
    /// A decimal string is malformed or out of range.
    InvalidDecimal(String),
    /// A field type expression is malformed.
    InvalidType(String),
    /// The schema definition itself is inconsistent.
    InvalidSchema(String),
    /// A state or patch does not conform to its schema.
    SchemaViolation { path: String, reason: String },
    /// Checked arithmetic overflowed or mixed decimal scales.
    Arithmetic,
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelError::InvalidId(s) => write!(f, "invalid identifier: {s}"),
            ModelError::InvalidDecimal(s) => write!(f, "invalid decimal: {s}"),
            ModelError::InvalidType(s) => write!(f, "invalid field type: {s}"),
            ModelError::InvalidSchema(s) => write!(f, "invalid schema: {s}"),
            ModelError::SchemaViolation { path, reason } => {
                write!(f, "schema violation at {path}: {reason}")
            }
            ModelError::Arithmetic => write!(f, "arithmetic overflow or scale mismatch"),
        }
    }
}

impl std::error::Error for ModelError {}
