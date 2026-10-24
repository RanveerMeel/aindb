//! Schemas, objects and state validation (SPEC §4.4).

use crate::{check_name, FieldType, ModelError, ObjectId, SchemaId, Value, Version};
use std::collections::{BTreeMap, BTreeSet};

/// Maximum number of fields per schema.
pub const MAX_FIELDS: usize = 256;
/// Maximum number of fields in one index.
pub const MAX_INDEX_FIELDS: usize = 8;

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

fn violation(path: &str, reason: impl Into<String>) -> ModelError {
    ModelError::SchemaViolation {
        path: path.to_owned(),
        reason: reason.into(),
    }
}

fn unique<'a>(what: &str, names: impl Iterator<Item = &'a str>) -> Result<(), ModelError> {
    let mut seen = BTreeSet::new();
    for n in names {
        check_name(n)?;
        if !seen.insert(n) {
            return Err(ModelError::InvalidSchema(format!("duplicate {what} {n:?}")));
        }
    }
    Ok(())
}

impl Schema {
    pub fn field(&self, name: &str) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Checks that the schema definition itself is consistent.
    pub fn check(&self) -> Result<(), ModelError> {
        let invalid = |m: String| Err(ModelError::InvalidSchema(m));
        if self.fields.is_empty() || self.fields.len() > MAX_FIELDS {
            return invalid(format!("{}: 1..={MAX_FIELDS} fields required", self.id));
        }
        unique("field", self.fields.iter().map(|f| f.name.as_str()))?;
        unique("index", self.indexes.iter().map(|i| i.name.as_str()))?;
        unique("invariant", self.invariants.iter().map(|i| i.name.as_str()))?;
        for f in &self.fields {
            if let Some(d) = &f.default {
                if *d == Value::Null {
                    return invalid(format!("field {:?}: null default", f.name));
                }
                f.ty.check(d, &f.name)?;
            }
        }
        for ix in &self.indexes {
            if ix.fields.is_empty() || ix.fields.len() > MAX_INDEX_FIELDS {
                return invalid(format!(
                    "index {:?}: 1..={MAX_INDEX_FIELDS} fields",
                    ix.name
                ));
            }
            let mut seen = BTreeSet::new();
            for name in &ix.fields {
                let f = self.field(name).ok_or_else(|| {
                    ModelError::InvalidSchema(format!("index {:?}: no field {name:?}", ix.name))
                })?;
                if matches!(f.ty, FieldType::List(_) | FieldType::Map(_)) {
                    return invalid(format!("index {:?}: field {name:?} is not scalar", ix.name));
                }
                if !seen.insert(name) {
                    return invalid(format!("index {:?}: field {name:?} repeated", ix.name));
                }
            }
        }
        if let Some(i) = self.invariants.iter().find(|i| i.expr.trim().is_empty()) {
            return invalid(format!("invariant {:?}: empty expression", i.name));
        }
        Ok(())
    }

    /// Validates the full state of a new object and fills in defaults.
    /// Explicit nulls on optional fields are dropped.
    pub fn validate_state(
        &self,
        state: &BTreeMap<String, Value>,
    ) -> Result<BTreeMap<String, Value>, ModelError> {
        self.reject_unknown(state)?;
        let mut out = BTreeMap::new();
        for f in &self.fields {
            match state.get(&f.name) {
                Some(Value::Null) | None => {
                    if let Some(d) = &f.default {
                        out.insert(f.name.clone(), d.clone());
                    } else if f.required {
                        return Err(violation(&f.name, "required field missing"));
                    }
                }
                Some(v) => {
                    f.ty.check(v, &f.name)?;
                    out.insert(f.name.clone(), v.clone());
                }
            }
        }
        Ok(out)
    }

    /// Validates a field-level patch. `Null` removes an optional field.
    pub fn validate_patch(&self, patch: &BTreeMap<String, Value>) -> Result<(), ModelError> {
        self.reject_unknown(patch)?;
        for (name, v) in patch {
            let f = self.field(name).expect("checked by reject_unknown");
            if *v == Value::Null {
                if f.required {
                    return Err(violation(name, "required field cannot be removed"));
                }
            } else {
                f.ty.check(v, name)?;
            }
        }
        Ok(())
    }

    fn reject_unknown(&self, state: &BTreeMap<String, Value>) -> Result<(), ModelError> {
        match state.keys().find(|k| self.field(k).is_none()) {
            Some(k) => Err(violation(k, format!("not a field of {}", self.id))),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Decimal;

    fn field(name: &str, ty: &str, required: bool) -> FieldDef {
        FieldDef {
            name: name.into(),
            ty: FieldType::parse(ty).unwrap(),
            required,
            default: None,
            sensitivity: Sensitivity::Public,
        }
    }

    fn account() -> Schema {
        Schema {
            id: SchemaId::parse("account@1").unwrap(),
            fields: vec![
                field("owner", "text", true),
                field("balance", "decimal(2)", true),
                FieldDef {
                    default: Some(Value::Text("INR".into())),
                    ..field("currency", "text", true)
                },
                field("tags", "list(text)", false),
            ],
            indexes: vec![IndexDef {
                name: "by_owner".into(),
                fields: vec!["owner".into()],
                unique: false,
            }],
            invariants: vec![InvariantDef {
                name: "non_negative".into(),
                expr: "balance >= 0".into(),
            }],
        }
    }

    fn state(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn schema_definition_checks() {
        assert_eq!(account().check(), Ok(()));
        let mut s = account();
        s.fields.push(field("owner", "text", false));
        assert!(matches!(s.check(), Err(ModelError::InvalidSchema(_))));
        let mut s = account();
        s.indexes[0].fields = vec!["tags".into()];
        assert!(s.check().is_err(), "list fields are not indexable");
        let mut s = account();
        s.indexes[0].fields = vec!["missing".into()];
        assert!(s.check().is_err());
    }

    #[test]
    fn state_validation_applies_defaults_and_types() {
        let s = account();
        let bal = Value::Decimal(Decimal::parse("10.00").unwrap());
        let out = s
            .validate_state(&state(&[
                ("owner", Value::Text("asha".into())),
                ("balance", bal.clone()),
            ]))
            .unwrap();
        assert_eq!(out.get("currency"), Some(&Value::Text("INR".into())));

        let err = s
            .validate_state(&state(&[("owner", Value::Text("asha".into()))]))
            .unwrap_err();
        assert!(matches!(err, ModelError::SchemaViolation { ref path, .. } if path == "balance"));

        let wrong_scale = Value::Decimal(Decimal::parse("10.0").unwrap());
        let err = s
            .validate_state(&state(&[
                ("owner", Value::Text("a".into())),
                ("balance", wrong_scale),
            ]))
            .unwrap_err();
        assert!(matches!(err, ModelError::SchemaViolation { ref path, .. } if path == "balance"));

        let err = s
            .validate_state(&state(&[
                ("owner", Value::Text("a".into())),
                ("balance", bal.clone()),
                (
                    "tags",
                    Value::List(vec![Value::Text("x".into()), Value::I64(1)]),
                ),
            ]))
            .unwrap_err();
        assert!(matches!(err, ModelError::SchemaViolation { ref path, .. } if path == "tags[1]"));

        let err = s
            .validate_patch(&state(&[("nickname", Value::Text("x".into()))]))
            .unwrap_err();
        assert!(matches!(err, ModelError::SchemaViolation { ref path, .. } if path == "nickname"));
        assert!(s.validate_patch(&state(&[("owner", Value::Null)])).is_err());
        assert!(s.validate_patch(&state(&[("tags", Value::Null)])).is_ok());
    }
}
