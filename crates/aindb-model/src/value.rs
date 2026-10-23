//! Values and field types (SPEC §4.3). No floating point: money and
//! comparisons must be exact and deterministic.

use crate::{ModelError, ObjectId, SchemaId};
use std::collections::BTreeMap;
use std::fmt;

/// Largest supported decimal scale (fractional digits).
pub const MAX_SCALE: u8 = 18;

/// Fixed-point decimal: `units * 10^-scale`. Arithmetic is always checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decimal {
    pub units: i128,
    pub scale: u8,
}

impl Decimal {
    /// Parses the canonical form: optional `-`, integer digits without
    /// leading zeros, and exactly `scale` fractional digits when scale > 0
    /// (`"10000.00"`, `"-1.5"`, `"0"`). Negative zero is rejected.
    pub fn parse(s: &str) -> Result<Decimal, ModelError> {
        let bad = || ModelError::InvalidDecimal(s.to_owned());
        let (neg, body) = match s.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, s),
        };
        let (int, frac) = match body.split_once('.') {
            Some((i, f)) if !f.is_empty() => (i, f),
            Some(_) => return Err(bad()),
            None => (body, ""),
        };
        let digits = |p: &str| !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit());
        if !digits(int) || (int.len() > 1 && int.starts_with('0')) {
            return Err(bad());
        }
        if !frac.is_empty() && !digits(frac) {
            return Err(bad());
        }
        let scale = u8::try_from(frac.len()).map_err(|_| bad())?;
        if scale > MAX_SCALE {
            return Err(bad());
        }
        let mut units: i128 = 0;
        for c in int.bytes().chain(frac.bytes()) {
            units = units
                .checked_mul(10)
                .and_then(|u| u.checked_add(i128::from(c - b'0')))
                .ok_or_else(bad)?;
        }
        if neg {
            if units == 0 {
                return Err(bad());
            }
            units = -units;
        }
        Ok(Decimal { units, scale })
    }

    /// Checked addition of two decimals with the same scale.
    pub fn checked_add(self, rhs: Decimal) -> Result<Decimal, ModelError> {
        if self.scale != rhs.scale {
            return Err(ModelError::Arithmetic);
        }
        let units = self
            .units
            .checked_add(rhs.units)
            .ok_or(ModelError::Arithmetic)?;
        Ok(Decimal {
            units,
            scale: self.scale,
        })
    }

    /// Checked subtraction of two decimals with the same scale.
    pub fn checked_sub(self, rhs: Decimal) -> Result<Decimal, ModelError> {
        if self.scale != rhs.scale {
            return Err(ModelError::Arithmetic);
        }
        let units = self
            .units
            .checked_sub(rhs.units)
            .ok_or(ModelError::Arithmetic)?;
        Ok(Decimal {
            units,
            scale: self.scale,
        })
    }
}

impl fmt::Display for Decimal {
    /// Canonical form; `Decimal::parse(&d.to_string()) == Ok(d)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.units < 0 { "-" } else { "" };
        let abs = self.units.unsigned_abs();
        if self.scale == 0 {
            return write!(f, "{sign}{abs}");
        }
        let pow = 10u128.pow(u32::from(self.scale));
        let width = usize::from(self.scale);
        write!(f, "{sign}{}.{:0width$}", abs / pow, abs % pow)
    }
}

/// Attribute value (SPEC §4.3).
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

impl Value {
    /// Short type name for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::I64(_) => "i64",
            Value::U64(_) => "u64",
            Value::Decimal(_) => "decimal",
            Value::Text(_) => "text",
            Value::Bytes(_) => "bytes",
            Value::Timestamp(_) => "timestamp",
            Value::Ref(_) => "ref",
            Value::List(_) => "list",
            Value::Map(_) => "map",
        }
    }
}

/// Declared type of a schema field. Text form (used in schemas and the
/// typed protocol): `bool`, `i64`, `u64`, `decimal(2)`, `text`, `bytes`,
/// `timestamp`, `ref(account@1)`, `list(<type>)`, `map(<type>)`.
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

impl FieldType {
    pub fn parse(s: &str) -> Result<FieldType, ModelError> {
        Self::parse_depth(s, 0)
    }

    fn parse_depth(s: &str, depth: usize) -> Result<FieldType, ModelError> {
        let bad = || ModelError::InvalidType(s.to_owned());
        if depth > 8 {
            return Err(bad());
        }
        let simple = match s {
            "bool" => Some(FieldType::Bool),
            "i64" => Some(FieldType::I64),
            "u64" => Some(FieldType::U64),
            "text" => Some(FieldType::Text),
            "bytes" => Some(FieldType::Bytes),
            "timestamp" => Some(FieldType::Timestamp),
            _ => None,
        };
        if let Some(t) = simple {
            return Ok(t);
        }
        let (head, arg) = s
            .strip_suffix(')')
            .and_then(|r| r.split_once('('))
            .ok_or_else(bad)?;
        match head {
            "decimal" => {
                if arg.is_empty() || (arg.len() > 1 && arg.starts_with('0')) {
                    return Err(bad());
                }
                let scale: u8 = arg.parse().map_err(|_| bad())?;
                if scale > MAX_SCALE {
                    return Err(bad());
                }
                Ok(FieldType::Decimal { scale })
            }
            "ref" => Ok(FieldType::Ref(SchemaId::parse(arg)?)),
            "list" => Ok(FieldType::List(Box::new(Self::parse_depth(
                arg,
                depth + 1,
            )?))),
            "map" => Ok(FieldType::Map(Box::new(Self::parse_depth(arg, depth + 1)?))),
            _ => Err(bad()),
        }
    }

    /// Checks `v` against this type. `Null` is handled by the caller
    /// (required/optional), so it never matches here.
    pub fn check(&self, v: &Value, path: &str) -> Result<(), ModelError> {
        let mismatch = || ModelError::SchemaViolation {
            path: path.to_owned(),
            reason: format!("expected {self}, got {}", v.type_name()),
        };
        match (self, v) {
            (FieldType::Bool, Value::Bool(_))
            | (FieldType::I64, Value::I64(_))
            | (FieldType::U64, Value::U64(_))
            | (FieldType::Text, Value::Text(_))
            | (FieldType::Bytes, Value::Bytes(_))
            | (FieldType::Timestamp, Value::Timestamp(_))
            // The referenced object's schema is checked by the engine.
            | (FieldType::Ref(_), Value::Ref(_)) => Ok(()),
            (FieldType::Decimal { scale }, Value::Decimal(d)) => {
                if d.scale == *scale {
                    Ok(())
                } else {
                    Err(ModelError::SchemaViolation {
                        path: path.to_owned(),
                        reason: format!("expected {scale} fractional digits, got {}", d.scale),
                    })
                }
            }
            (FieldType::List(t), Value::List(items)) => items
                .iter()
                .enumerate()
                .try_for_each(|(i, x)| t.check(x, &format!("{path}[{i}]"))),
            (FieldType::Map(t), Value::Map(m)) => m
                .iter()
                .try_for_each(|(k, x)| t.check(x, &format!("{path}.{k}"))),
            _ => Err(mismatch()),
        }
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldType::Bool => f.write_str("bool"),
            FieldType::I64 => f.write_str("i64"),
            FieldType::U64 => f.write_str("u64"),
            FieldType::Decimal { scale } => write!(f, "decimal({scale})"),
            FieldType::Text => f.write_str("text"),
            FieldType::Bytes => f.write_str("bytes"),
            FieldType::Timestamp => f.write_str("timestamp"),
            FieldType::Ref(s) => write!(f, "ref({s})"),
            FieldType::List(t) => write!(f, "list({t})"),
            FieldType::Map(t) => write!(f, "map({t})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_parse_display_round_trip() {
        for s in ["0", "1", "-1", "10000.00", "-0.05", "0.000000000000000001"] {
            assert_eq!(Decimal::parse(s).unwrap().to_string(), s);
        }
        assert_eq!(
            Decimal::parse("10000.00").unwrap(),
            Decimal {
                units: 1_000_000,
                scale: 2
            }
        );
        for bad in [
            "",
            "-",
            "+1",
            "01",
            "1.",
            ".5",
            "-0",
            "-0.00",
            "1e3",
            "1.2.3",
            "0.0000000000000000001",
        ] {
            assert!(Decimal::parse(bad).is_err(), "{bad}");
        }
        assert!(Decimal::parse(&"9".repeat(40)).is_err(), "overflow");
    }

    #[test]
    fn decimal_arithmetic_is_checked() {
        let max = Decimal {
            units: i128::MAX,
            scale: 2,
        };
        let one = Decimal { units: 1, scale: 2 };
        assert_eq!(max.checked_add(one), Err(ModelError::Arithmetic));
        assert_eq!(
            one.checked_add(Decimal { units: 1, scale: 3 }),
            Err(ModelError::Arithmetic)
        );
        assert_eq!(one.checked_sub(one), Ok(Decimal { units: 0, scale: 2 }));
    }

    #[test]
    fn field_types_round_trip() {
        for s in [
            "bool",
            "decimal(2)",
            "ref(account@1)",
            "list(map(decimal(0)))",
        ] {
            assert_eq!(FieldType::parse(s).unwrap().to_string(), s);
        }
        for bad in [
            "float",
            "decimal()",
            "decimal(02)",
            "decimal(19)",
            "list(",
            "ref(x)",
            "list(text",
        ] {
            assert!(FieldType::parse(bad).is_err(), "{bad}");
        }
    }
}
