//! Shared fixtures for the acceptance tests in `tests/` (docs/ACCEPTANCE.md).
//!
//! * [`account_schema`] — the `account@1` fixture used throughout.
//! * [`Gen`] — a small deterministic generator of valid requests, so
//!   property-style tests are reproducible without extra dependencies.

use aindb_model::{
    Decimal, FieldDef, FieldType, IndexDef, InvariantDef, ObjectId, Schema, SchemaId, Sensitivity,
    TxId, Value,
};
use aindb_protocol::{
    HistorySelector, Intent, Operation, Options, Precondition, PrincipalClaim, Request,
};
use std::collections::BTreeMap;

/// `account@1 {owner: text, balance: decimal(2), currency: text = "INR"}`
/// with invariant `balance >= 0`.
pub fn account_schema() -> Schema {
    let field = |name: &str, ty: &str, required| FieldDef {
        name: name.into(),
        ty: FieldType::parse(ty).unwrap(),
        required,
        default: None,
        sensitivity: Sensitivity::Public,
    };
    Schema {
        id: SchemaId::parse("account@1").unwrap(),
        fields: vec![
            field("owner", "text", true),
            field("balance", "decimal(2)", true),
            FieldDef {
                default: Some(Value::Text("INR".into())),
                ..field("currency", "text", true)
            },
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

pub fn dec(s: &str) -> Value {
    Value::Decimal(Decimal::parse(s).unwrap())
}

pub fn fields(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

/// A minimal valid request around the given operations.
pub fn request(ops: Vec<Operation>) -> Request {
    Request {
        tx_id: TxId::parse("tx-1").unwrap(),
        principal: PrincipalClaim::ApiKey("k-agent-17".into()),
        ops,
        preconditions: vec![],
        intent: Intent::default(),
        options: Options::default(),
    }
}

/// Deterministic xorshift generator of valid protocol values.
pub struct Gen(u64);

impl Gen {
    pub fn new(seed: u64) -> Self {
        Gen(seed.max(1))
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
        xs[self.below(xs.len() as u64) as usize]
    }

    pub fn name(&mut self) -> String {
        format!(
            "{}_{}",
            self.pick(&["owner", "balance", "tags", "note", "x"]),
            self.below(5)
        )
    }

    /// Text exercising JSON escaping: quotes, backslashes, controls, non-ASCII.
    pub fn text(&mut self) -> String {
        let parts = [
            "a", "Z", " ", "\"", "\\", "\n", "\u{1}", "é", "😀", "/", "$",
        ];
        (0..self.below(8)).map(|_| self.pick(&parts)).collect()
    }

    pub fn object_id(&mut self) -> ObjectId {
        let ns = self.pick(&["acct", "hr", "doc-1", "a_b"]);
        ObjectId::new(
            ns,
            &format!("K{}.{}", self.below(1000), self.pick(&["x", "y-z", "_"])),
        )
        .unwrap()
    }

    pub fn value(&mut self, depth: u32) -> Value {
        let kinds = if depth >= 3 { 9 } else { 11 };
        match self.below(kinds) {
            0 => Value::Null,
            1 => Value::Bool(self.below(2) == 1),
            2 => Value::I64(self.next_u64() as i64),
            3 => Value::U64(self.next_u64()),
            4 => Value::Decimal(Decimal {
                units: (self.next_u64() as i64) as i128 * 1_000_003,
                scale: self.below(19) as u8,
            }),
            5 => Value::Text(self.text()),
            6 => Value::Bytes((0..self.below(6)).map(|_| self.next_u64() as u8).collect()),
            7 => {
                Value::Timestamp(self.next_u64() as i128 * if self.below(2) == 0 { -1 } else { 1 })
            }
            8 => Value::Ref(self.object_id()),
            9 => Value::List((0..self.below(4)).map(|_| self.value(depth + 1)).collect()),
            _ => Value::Map(self.map(depth + 1)),
        }
    }

    pub fn map(&mut self, depth: u32) -> BTreeMap<String, Value> {
        (0..self.below(4))
            .map(|_| (self.name(), self.value(depth)))
            .collect()
    }

    fn opt(&mut self) -> Option<u64> {
        (self.below(2) == 0).then(|| self.next_u64())
    }

    pub fn operation(&mut self) -> Operation {
        match self.below(8) {
            0 => Operation::Create {
                schema: SchemaId::new("account", 1 + self.below(3) as u16).unwrap(),
                id: self.object_id(),
                state: self.map(0),
            },
            1 => Operation::Update {
                id: self.object_id(),
                expect_version: self.opt(),
                patch: self.map(0),
            },
            2 => Operation::Delete {
                id: self.object_id(),
                expect_version: self.opt(),
            },
            3 => Operation::Get {
                id: self.object_id(),
                at: self.opt(),
            },
            4 => Operation::Scan {
                index: "by_owner".into(),
                start: vec![self.value(2)],
                end: vec![self.value(2)],
                limit: 1 + self.below(10_000) as u32,
            },
            5 => Operation::History {
                selector: match self.below(3) {
                    0 => HistorySelector::Object(self.object_id()),
                    1 => HistorySelector::Tx(
                        TxId::parse(&format!("tx-{}", self.next_u64())).unwrap(),
                    ),
                    _ => HistorySelector::Principal(self.text()),
                },
                from: self.next_u64(),
                limit: 1 + self.below(10_000) as u32,
            },
            6 => {
                let mut s = account_schema();
                if self.below(2) == 0 {
                    s.fields[0].sensitivity = Sensitivity::Secret;
                    s.fields[0].default = Some(Value::Text(self.text()));
                }
                Operation::DefineSchema(s)
            }
            _ => Operation::Named {
                op: SchemaId::parse("transfer@1").unwrap(),
                inputs: self.map(0),
            },
        }
    }

    pub fn request(&mut self) -> Request {
        let principal = match self.below(3) {
            0 => PrincipalClaim::ApiKey(self.text()),
            1 => PrincipalClaim::Ed25519 {
                key_id: self.text(),
                signature: (0..self.below(64)).map(|_| self.next_u64() as u8).collect(),
            },
            _ => PrincipalClaim::Cdk((0..self.below(64)).map(|_| self.next_u64() as u8).collect()),
        };
        let preconditions = (0..self.below(3))
            .map(|_| match self.below(3) {
                0 => Precondition::Exists(self.object_id()),
                1 => Precondition::NotExists(self.object_id()),
                _ => Precondition::VersionIs(self.object_id(), self.next_u64()),
            })
            .collect();
        Request {
            tx_id: TxId::parse(&format!("tx-{}", self.next_u64())).unwrap(),
            principal,
            ops: (0..1 + self.below(5)).map(|_| self.operation()).collect(),
            preconditions,
            intent: match self.below(3) {
                0 => Intent::default(),
                1 => Intent {
                    reason: self.text(),
                    request_ref: None,
                },
                _ => Intent {
                    reason: self.text(),
                    request_ref: Some(self.text()),
                },
            },
            options: Options {
                dry_run: self.below(2) == 0,
                want_explanation: self.below(2) == 0,
            },
        }
    }
}
