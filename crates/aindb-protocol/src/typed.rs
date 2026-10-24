//! Typed (JSON) form of `AINDB-OP-v1` (ADR-0004, docs/PROTOCOL.md).
//!
//! Values map to JSON as: `null`, `true`/`false`, integers → `i64`,
//! strings → `text`, arrays → `list`, objects → `map`. Every other type is a
//! one-key tagged object: `{"$u64": 5}`, `{"$decimal": "10.00"}`,
//! `{"$bytes": "00ff"}`, `{"$timestamp": 1700000000000000000}`,
//! `{"$ref": "acct:A-100"}`. Map keys must not start with `$`.

use crate::json::{self, Json};
use crate::{
    check_limits, limits, Codec, DecodeError, ErrorCode, HistorySelector, Intent, Operation,
    Options, Precondition, PrincipalClaim, Request, Status, TxResult, PROTOCOL,
};
use aindb_model::{
    Decimal, FieldDef, FieldType, IndexDef, InvariantDef, ModelError, ObjectId, Schema, SchemaId,
    Sensitivity, TxId, Value,
};
use std::collections::BTreeMap;

/// The strict JSON codec.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypedJson;

impl Codec for TypedJson {
    fn decode_request(&self, bytes: &[u8]) -> Result<Request, DecodeError> {
        if bytes.len() > limits::MAX_TX_BYTES {
            return Err(DecodeError::LimitExceeded(format!(
                "request is {} bytes, max {}",
                bytes.len(),
                limits::MAX_TX_BYTES
            )));
        }
        let req = request(json::parse(bytes)?)?;
        check_limits(&req)?;
        Ok(req)
    }

    fn encode_request(&self, req: &Request) -> Vec<u8> {
        let mut out = String::new();
        json::encode(&request_json(req), &mut out);
        out.into_bytes()
    }

    fn encode_result(&self, res: &TxResult) -> Vec<u8> {
        let mut out = String::new();
        json::encode(&result_json(res), &mut out);
        out.into_bytes()
    }
}

// ---------------------------------------------------------------- decoding

fn invalid(path: &str, what: impl std::fmt::Display) -> DecodeError {
    DecodeError::Invalid(format!("{path}: {what}"))
}

fn model(path: &str) -> impl Fn(ModelError) -> DecodeError + '_ {
    move |e| invalid(path, e)
}

/// An object being consumed field by field; leftovers are unknown fields.
struct Fields {
    path: String,
    members: Vec<(String, Option<Json>)>,
}

impl Fields {
    fn new(j: Json, path: &str) -> Result<Fields, DecodeError> {
        match j {
            Json::Obj(m) => Ok(Fields {
                path: path.to_owned(),
                members: m.into_iter().map(|(k, v)| (k, Some(v))).collect(),
            }),
            _ => Err(invalid(path, "expected an object")),
        }
    }

    fn child(&self, key: &str) -> String {
        format!("{}.{key}", self.path)
    }

    fn opt(&mut self, key: &str) -> Option<Json> {
        self.members
            .iter_mut()
            .find(|(k, _)| k == key)
            .and_then(|(_, v)| v.take())
    }

    fn req(&mut self, key: &str) -> Result<Json, DecodeError> {
        self.opt(key)
            .ok_or_else(|| DecodeError::Invalid(format!("{}: missing field", self.child(key))))
    }

    fn finish(self) -> Result<(), DecodeError> {
        match self.members.into_iter().find(|(_, v)| v.is_some()) {
            Some((k, _)) => Err(DecodeError::UnknownField(format!("{}.{k}", self.path))),
            None => Ok(()),
        }
    }
}

/// `{"<tag>": value}` with exactly one member.
fn variant(j: Json, path: &str) -> Result<(String, Json, String), DecodeError> {
    match j {
        Json::Obj(mut m) if m.len() == 1 => {
            let (k, v) = m.pop().expect("one member");
            let p = format!("{path}.{k}");
            Ok((k, v, p))
        }
        _ => Err(invalid(path, "expected an object with exactly one key")),
    }
}

fn string(j: Json, path: &str) -> Result<String, DecodeError> {
    match j {
        Json::Str(s) => Ok(s),
        _ => Err(invalid(path, "expected a string")),
    }
}

fn boolean(j: Json, path: &str) -> Result<bool, DecodeError> {
    match j {
        Json::Bool(b) => Ok(b),
        _ => Err(invalid(path, "expected true or false")),
    }
}

fn int<T: TryFrom<i128>>(j: Json, path: &str) -> Result<T, DecodeError> {
    match j {
        Json::Int(n) => T::try_from(n).map_err(|_| invalid(path, "integer out of range")),
        _ => Err(invalid(path, "expected an integer")),
    }
}

fn array(j: Json, path: &str) -> Result<Vec<Json>, DecodeError> {
    match j {
        Json::Arr(a) => Ok(a),
        _ => Err(invalid(path, "expected an array")),
    }
}

fn hex(s: &str, path: &str) -> Result<Vec<u8>, DecodeError> {
    let b = s.as_bytes();
    if b.len() % 2 != 0
        || !b
            .iter()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c))
    {
        return Err(DecodeError::NonCanonical(format!(
            "{path}: expected lowercase hex"
        )));
    }
    Ok(b.chunks(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).expect("ASCII"), 16).expect("hex"))
        .collect())
}

fn object_id(j: Json, path: &str) -> Result<ObjectId, DecodeError> {
    ObjectId::parse(&string(j, path)?).map_err(model(path))
}

fn versioned(j: Json, path: &str) -> Result<SchemaId, DecodeError> {
    SchemaId::parse(&string(j, path)?).map_err(model(path))
}

fn value(j: Json, path: &str) -> Result<Value, DecodeError> {
    Ok(match j {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(b),
        Json::Int(n) => Value::I64(i64::try_from(n).map_err(|_| {
            invalid(
                path,
                "integer outside i64; use {\"$u64\": n} or {\"$decimal\": \"…\"}",
            )
        })?),
        Json::Str(s) => Value::Text(s),
        Json::Arr(items) => Value::List(
            items
                .into_iter()
                .enumerate()
                .map(|(i, x)| value(x, &format!("{path}.{i}")))
                .collect::<Result<_, _>>()?,
        ),
        Json::Obj(m) if m.iter().any(|(k, _)| k.starts_with('$')) => {
            let (tag, v, p) = variant(Json::Obj(m), path)?;
            match tag.as_str() {
                "$u64" => Value::U64(int(v, &p)?),
                "$decimal" => Value::Decimal(Decimal::parse(&string(v, &p)?).map_err(model(&p))?),
                "$bytes" => Value::Bytes(hex(&string(v, &p)?, &p)?),
                "$timestamp" => Value::Timestamp(int(v, &p)?),
                "$ref" => Value::Ref(object_id(v, &p)?),
                _ => return Err(DecodeError::UnknownField(p)),
            }
        }
        Json::Obj(m) => Value::Map(map(m, path)?),
    })
}

fn map(members: Vec<(String, Json)>, path: &str) -> Result<BTreeMap<String, Value>, DecodeError> {
    members
        .into_iter()
        .map(|(k, v)| {
            let p = format!("{path}.{k}");
            Ok((k, value(v, &p)?))
        })
        .collect()
}

/// An object state or patch, bounded by `MAX_OBJECT_BYTES` of canonical
/// typed encoding so the limit does not depend on whitespace.
fn state(j: Json, path: &str) -> Result<BTreeMap<String, Value>, DecodeError> {
    let m = match j {
        Json::Obj(m) if !m.iter().any(|(k, _)| k.starts_with('$')) => map(m, path)?,
        _ => return Err(invalid(path, "expected an object of fields")),
    };
    let mut out = String::new();
    json::encode(&map_json(&m), &mut out);
    if out.len() > limits::MAX_OBJECT_BYTES {
        return Err(DecodeError::LimitExceeded(format!(
            "{path}: object state is {} bytes, max {}",
            out.len(),
            limits::MAX_OBJECT_BYTES
        )));
    }
    Ok(m)
}

fn values(j: Json, path: &str) -> Result<Vec<Value>, DecodeError> {
    array(j, path)?
        .into_iter()
        .enumerate()
        .map(|(i, x)| value(x, &format!("{path}.{i}")))
        .collect()
}

fn request(j: Json) -> Result<Request, DecodeError> {
    let mut f = Fields::new(j, "$")?;
    let proto = string(f.req("protocol")?, "$.protocol")?;
    if proto != PROTOCOL {
        return Err(DecodeError::UnsupportedVersion(proto));
    }
    let tx_id = TxId::parse(&string(f.req("tx_id")?, "$.tx_id")?).map_err(model("$.tx_id"))?;
    let principal = principal(f.req("principal")?)?;
    let raw_ops = array(f.req("ops")?, "$.ops")?;
    if raw_ops.len() > limits::MAX_OPS {
        return Err(DecodeError::LimitExceeded(format!(
            "{} operations, max {}",
            raw_ops.len(),
            limits::MAX_OPS
        )));
    }
    let ops = raw_ops
        .into_iter()
        .enumerate()
        .map(|(i, o)| operation(o, &format!("$.ops.{i}")))
        .collect::<Result<_, _>>()?;
    let preconditions = match f.opt("preconditions") {
        Some(p) => array(p, "$.preconditions")?
            .into_iter()
            .enumerate()
            .map(|(i, x)| precondition(x, &format!("$.preconditions.{i}")))
            .collect::<Result<_, _>>()?,
        None => Vec::new(),
    };
    let intent = match f.opt("intent") {
        Some(j) => {
            let mut g = Fields::new(j, "$.intent")?;
            let reason = string(g.req("reason")?, "$.intent.reason")?;
            let request_ref = g
                .opt("request_ref")
                .map(|r| string(r, "$.intent.request_ref"))
                .transpose()?;
            g.finish()?;
            Intent {
                reason,
                request_ref,
            }
        }
        None => Intent::default(),
    };
    let options = match f.opt("options") {
        Some(j) => {
            let mut g = Fields::new(j, "$.options")?;
            let dry_run = g
                .opt("dry_run")
                .map(|b| boolean(b, "$.options.dry_run"))
                .transpose()?;
            let want = g
                .opt("want_explanation")
                .map(|b| boolean(b, "$.options.want_explanation"))
                .transpose()?;
            g.finish()?;
            Options {
                dry_run: dry_run.unwrap_or(false),
                want_explanation: want.unwrap_or(false),
            }
        }
        None => Options::default(),
    };
    f.finish()?;
    Ok(Request {
        tx_id,
        principal,
        ops,
        preconditions,
        intent,
        options,
    })
}

fn principal(j: Json) -> Result<PrincipalClaim, DecodeError> {
    let (tag, v, p) = variant(j, "$.principal")?;
    Ok(match tag.as_str() {
        "api_key" => PrincipalClaim::ApiKey(string(v, &p)?),
        "ed25519" => {
            let mut f = Fields::new(v, &p)?;
            let key_id = string(f.req("key_id")?, &f.child("key_id"))?;
            let sp = f.child("signature");
            let signature = hex(&string(f.req("signature")?, &sp)?, &sp)?;
            f.finish()?;
            PrincipalClaim::Ed25519 { key_id, signature }
        }
        "cdk" => PrincipalClaim::Cdk(hex(&string(v, &p)?, &p)?),
        _ => return Err(DecodeError::UnknownField(p)),
    })
}

fn precondition(j: Json, path: &str) -> Result<Precondition, DecodeError> {
    let (tag, v, p) = variant(j, path)?;
    Ok(match tag.as_str() {
        "exists" => Precondition::Exists(object_id(v, &p)?),
        "not_exists" => Precondition::NotExists(object_id(v, &p)?),
        "version_is" => {
            let mut f = Fields::new(v, &p)?;
            let id = object_id(f.req("id")?, &f.child("id"))?;
            let version = int(f.req("version")?, &f.child("version"))?;
            f.finish()?;
            Precondition::VersionIs(id, version)
        }
        _ => return Err(DecodeError::UnknownField(p)),
    })
}

fn operation(j: Json, path: &str) -> Result<Operation, DecodeError> {
    let (tag, v, p) = variant(j, path)?;
    if tag == "define_schema" {
        return schema(v, &p).map(Operation::DefineSchema);
    }
    let mut f = Fields::new(v, &p)?;
    let op = match tag.as_str() {
        "create" => Operation::Create {
            schema: versioned(f.req("schema")?, &f.child("schema"))?,
            id: object_id(f.req("id")?, &f.child("id"))?,
            state: state(f.req("state")?, &f.child("state"))?,
        },
        "update" => Operation::Update {
            id: object_id(f.req("id")?, &f.child("id"))?,
            expect_version: f
                .opt("expect_version")
                .map(|x| int(x, &f.child("expect_version")))
                .transpose()?,
            patch: state(f.req("patch")?, &f.child("patch"))?,
        },
        "delete" => Operation::Delete {
            id: object_id(f.req("id")?, &f.child("id"))?,
            expect_version: f
                .opt("expect_version")
                .map(|x| int(x, &f.child("expect_version")))
                .transpose()?,
        },
        "get" => Operation::Get {
            id: object_id(f.req("id")?, &f.child("id"))?,
            at: f.opt("at").map(|x| int(x, &f.child("at"))).transpose()?,
        },
        "scan" => Operation::Scan {
            index: string(f.req("index")?, &f.child("index"))?,
            start: values(f.req("start")?, &f.child("start"))?,
            end: values(f.req("end")?, &f.child("end"))?,
            limit: int(f.req("limit")?, &f.child("limit"))?,
        },
        "history" => {
            let sp = f.child("selector");
            let (stag, sv, spath) = variant(f.req("selector")?, &sp)?;
            let selector = match stag.as_str() {
                "object" => HistorySelector::Object(object_id(sv, &spath)?),
                "tx" => {
                    HistorySelector::Tx(TxId::parse(&string(sv, &spath)?).map_err(model(&spath))?)
                }
                "principal" => HistorySelector::Principal(string(sv, &spath)?),
                _ => return Err(DecodeError::UnknownField(spath)),
            };
            Operation::History {
                selector,
                from: f
                    .opt("from")
                    .map(|x| int(x, &f.child("from")))
                    .transpose()?
                    .unwrap_or(0),
                limit: int(f.req("limit")?, &f.child("limit"))?,
            }
        }
        "named" => Operation::Named {
            op: versioned(f.req("op")?, &f.child("op"))?,
            inputs: state(f.req("inputs")?, &f.child("inputs"))?,
        },
        _ => return Err(DecodeError::UnknownField(p)),
    };
    f.finish()?;
    Ok(op)
}

fn schema(j: Json, path: &str) -> Result<Schema, DecodeError> {
    let mut f = Fields::new(j, path)?;
    let id = versioned(f.req("id")?, &f.child("id"))?;
    let fields = array(f.req("fields")?, &f.child("fields"))?
        .into_iter()
        .enumerate()
        .map(|(i, x)| field_def(x, &format!("{path}.fields.{i}")))
        .collect::<Result<_, _>>()?;
    let indexes = match f.opt("indexes") {
        Some(a) => array(a, &f.child("indexes"))?
            .into_iter()
            .enumerate()
            .map(|(i, x)| {
                let ip = format!("{path}.indexes.{i}");
                let mut g = Fields::new(x, &ip)?;
                let name = string(g.req("name")?, &g.child("name"))?;
                let fp = g.child("fields");
                let fields = array(g.req("fields")?, &fp)?
                    .into_iter()
                    .map(|s| string(s, &fp))
                    .collect::<Result<_, _>>()?;
                let unique = g
                    .opt("unique")
                    .map(|b| boolean(b, &ip))
                    .transpose()?
                    .unwrap_or(false);
                g.finish()?;
                Ok(IndexDef {
                    name,
                    fields,
                    unique,
                })
            })
            .collect::<Result<_, DecodeError>>()?,
        None => Vec::new(),
    };
    let invariants = match f.opt("invariants") {
        Some(a) => array(a, &f.child("invariants"))?
            .into_iter()
            .enumerate()
            .map(|(i, x)| {
                let mut g = Fields::new(x, &format!("{path}.invariants.{i}"))?;
                let name = string(g.req("name")?, &g.child("name"))?;
                let expr = string(g.req("expr")?, &g.child("expr"))?;
                g.finish()?;
                Ok(InvariantDef { name, expr })
            })
            .collect::<Result<_, DecodeError>>()?,
        None => Vec::new(),
    };
    f.finish()?;
    Ok(Schema {
        id,
        fields,
        indexes,
        invariants,
    })
}

fn field_def(j: Json, path: &str) -> Result<FieldDef, DecodeError> {
    let mut f = Fields::new(j, path)?;
    let name = string(f.req("name")?, &f.child("name"))?;
    let tp = f.child("type");
    let ty = FieldType::parse(&string(f.req("type")?, &tp)?).map_err(model(&tp))?;
    let required = f
        .opt("required")
        .map(|b| boolean(b, path))
        .transpose()?
        .unwrap_or(false);
    let default = f.opt("default").map(|d| value(d, path)).transpose()?;
    let sp = f.child("sensitivity");
    let sensitivity = match f
        .opt("sensitivity")
        .map(|s| string(s, &sp))
        .transpose()?
        .as_deref()
    {
        None | Some("public") => Sensitivity::Public,
        Some("internal") => Sensitivity::Internal,
        Some("secret") => Sensitivity::Secret,
        Some(other) => return Err(invalid(&sp, format!("unknown sensitivity {other:?}"))),
    };
    f.finish()?;
    Ok(FieldDef {
        name,
        ty,
        required,
        default,
        sensitivity,
    })
}

// ---------------------------------------------------------------- encoding

fn obj(members: Vec<(&str, Json)>) -> Json {
    Json::Obj(
        members
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    )
}

fn s(x: impl ToString) -> Json {
    Json::Str(x.to_string())
}

fn n(x: impl Into<i128>) -> Json {
    Json::Int(x.into())
}

fn hex_str(b: &[u8]) -> Json {
    Json::Str(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Typed JSON form of a value (see module docs).
pub fn value_json(v: &Value) -> Json {
    match v {
        Value::Null => Json::Null,
        Value::Bool(b) => Json::Bool(*b),
        Value::I64(x) => n(*x),
        Value::U64(x) => obj(vec![("$u64", n(*x))]),
        Value::Decimal(d) => obj(vec![("$decimal", s(d))]),
        Value::Text(t) => s(t),
        Value::Bytes(b) => obj(vec![("$bytes", hex_str(b))]),
        Value::Timestamp(t) => obj(vec![("$timestamp", n(*t))]),
        Value::Ref(id) => obj(vec![("$ref", s(id))]),
        Value::List(items) => Json::Arr(items.iter().map(value_json).collect()),
        Value::Map(m) => map_json(m),
    }
}

fn map_json(m: &BTreeMap<String, Value>) -> Json {
    Json::Obj(m.iter().map(|(k, v)| (k.clone(), value_json(v))).collect())
}

fn principal_json(p: &PrincipalClaim) -> Json {
    match p {
        PrincipalClaim::ApiKey(k) => obj(vec![("api_key", s(k))]),
        PrincipalClaim::Ed25519 { key_id, signature } => obj(vec![(
            "ed25519",
            obj(vec![
                ("key_id", s(key_id)),
                ("signature", hex_str(signature)),
            ]),
        )]),
        PrincipalClaim::Cdk(t) => obj(vec![("cdk", hex_str(t))]),
    }
}

fn op_json(op: &Operation) -> Json {
    let opt_n = |m: &mut Vec<(&str, Json)>, k, v: Option<u64>| {
        if let Some(v) = v {
            m.push((k, n(v)));
        }
    };
    let (tag, body) = match op {
        Operation::Create { schema, id, state } => (
            "create",
            obj(vec![
                ("schema", s(schema)),
                ("id", s(id)),
                ("state", map_json(state)),
            ]),
        ),
        Operation::Update {
            id,
            expect_version,
            patch,
        } => {
            let mut m = vec![("id", s(id))];
            opt_n(&mut m, "expect_version", *expect_version);
            m.push(("patch", map_json(patch)));
            ("update", obj(m))
        }
        Operation::Delete { id, expect_version } => {
            let mut m = vec![("id", s(id))];
            opt_n(&mut m, "expect_version", *expect_version);
            ("delete", obj(m))
        }
        Operation::Get { id, at } => {
            let mut m = vec![("id", s(id))];
            opt_n(&mut m, "at", *at);
            ("get", obj(m))
        }
        Operation::Scan {
            index,
            start,
            end,
            limit,
        } => (
            "scan",
            obj(vec![
                ("index", s(index)),
                ("start", Json::Arr(start.iter().map(value_json).collect())),
                ("end", Json::Arr(end.iter().map(value_json).collect())),
                ("limit", n(*limit)),
            ]),
        ),
        Operation::History {
            selector,
            from,
            limit,
        } => {
            let sel = match selector {
                HistorySelector::Object(id) => obj(vec![("object", s(id))]),
                HistorySelector::Tx(t) => obj(vec![("tx", s(t))]),
                HistorySelector::Principal(p) => obj(vec![("principal", s(p))]),
            };
            (
                "history",
                obj(vec![
                    ("selector", sel),
                    ("from", n(*from)),
                    ("limit", n(*limit)),
                ]),
            )
        }
        Operation::DefineSchema(schema) => ("define_schema", schema_json(schema)),
        Operation::Named { op, inputs } => (
            "named",
            obj(vec![("op", s(op)), ("inputs", map_json(inputs))]),
        ),
    };
    obj(vec![(tag, body)])
}

fn schema_json(sc: &Schema) -> Json {
    let fields = sc
        .fields
        .iter()
        .map(|f| {
            let mut m = vec![
                ("name", s(&f.name)),
                ("type", s(&f.ty)),
                ("required", Json::Bool(f.required)),
            ];
            if let Some(d) = &f.default {
                m.push(("default", value_json(d)));
            }
            let sens = match f.sensitivity {
                Sensitivity::Public => "public",
                Sensitivity::Internal => "internal",
                Sensitivity::Secret => "secret",
            };
            m.push(("sensitivity", s(sens)));
            obj(m)
        })
        .collect();
    let indexes = sc
        .indexes
        .iter()
        .map(|i| {
            obj(vec![
                ("name", s(&i.name)),
                ("fields", Json::Arr(i.fields.iter().map(s).collect())),
                ("unique", Json::Bool(i.unique)),
            ])
        })
        .collect();
    let invariants = sc
        .invariants
        .iter()
        .map(|i| obj(vec![("name", s(&i.name)), ("expr", s(&i.expr))]))
        .collect();
    obj(vec![
        ("id", s(&sc.id)),
        ("fields", Json::Arr(fields)),
        ("indexes", Json::Arr(indexes)),
        ("invariants", Json::Arr(invariants)),
    ])
}

fn request_json(r: &Request) -> Json {
    let mut m = vec![
        ("protocol", s(PROTOCOL)),
        ("tx_id", s(&r.tx_id)),
        ("principal", principal_json(&r.principal)),
        ("ops", Json::Arr(r.ops.iter().map(op_json).collect())),
    ];
    if !r.preconditions.is_empty() {
        let pre = r
            .preconditions
            .iter()
            .map(|p| match p {
                Precondition::Exists(id) => obj(vec![("exists", s(id))]),
                Precondition::NotExists(id) => obj(vec![("not_exists", s(id))]),
                Precondition::VersionIs(id, v) => obj(vec![(
                    "version_is",
                    obj(vec![("id", s(id)), ("version", n(*v))]),
                )]),
            })
            .collect();
        m.push(("preconditions", Json::Arr(pre)));
    }
    if r.intent != Intent::default() {
        let mut i = vec![("reason", s(&r.intent.reason))];
        if let Some(rr) = &r.intent.request_ref {
            i.push(("request_ref", s(rr)));
        }
        m.push(("intent", obj(i)));
    }
    if r.options != Options::default() {
        m.push((
            "options",
            obj(vec![
                ("dry_run", Json::Bool(r.options.dry_run)),
                ("want_explanation", Json::Bool(r.options.want_explanation)),
            ]),
        ));
    }
    obj(m)
}

fn error_json(e: &ErrorCode) -> Json {
    let code = |c: &str| ("code", s(c));
    obj(match e {
        ErrorCode::Malformed(d) => vec![code("malformed"), ("detail", s(d))],
        ErrorCode::Unauthorized => vec![code("unauthorized")],
        ErrorCode::NeedsApproval => vec![code("needs_approval")],
        ErrorCode::SchemaViolation { field_path } => {
            vec![code("schema_violation"), ("field_path", s(field_path))]
        }
        ErrorCode::PreconditionFailed => vec![code("precondition_failed")],
        ErrorCode::InvariantViolated { name } => {
            vec![code("invariant_violated"), ("name", s(name))]
        }
        ErrorCode::Conflict => vec![code("conflict")],
        ErrorCode::VerifierRejected { verifier } => {
            vec![code("verifier_rejected"), ("verifier", s(verifier))]
        }
        ErrorCode::LimitExceeded => vec![code("limit_exceeded")],
        ErrorCode::Internal => vec![code("internal")],
    })
}

fn result_json(r: &TxResult) -> Json {
    let status = match r.status {
        Status::Committed => "committed",
        Status::Rejected => "rejected",
        Status::Aborted => "aborted",
        Status::DryRunOk => "dry_run_ok",
    };
    let versions = |v: &[(ObjectId, u64)]| {
        Json::Arr(
            v.iter()
                .map(|(id, ver)| obj(vec![("id", s(id)), ("version", n(*ver))]))
                .collect(),
        )
    };
    let mut m = vec![
        ("tx_id", s(&r.tx_id)),
        ("status", s(status)),
        ("reads", versions(&r.reads)),
        ("writes", versions(&r.writes)),
        (
            "values",
            Json::Arr(r.values.iter().map(value_json).collect()),
        ),
    ];
    if let Some(e) = &r.error {
        m.push(("error", error_json(e)));
    }
    if let Some(rc) = &r.receipt {
        let mut rm = vec![("seq", n(rc.seq)), ("event_hash", hex_str(&rc.event_hash))];
        if let Some(c) = rc.checkpoint_seq {
            rm.push(("checkpoint_seq", n(c)));
        }
        m.push(("receipt", obj(rm)));
    }
    if let Some(x) = &r.explanation {
        m.push(("explanation", s(x)));
    }
    obj(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRANSFER: &str = r#"{
      "protocol": "AINDB-OP-v1",
      "tx_id": "tx-1042",
      "principal": {"api_key": "k-agent-17"},
      "ops": [{"named": {"op": "transfer@1", "inputs": {
          "from": {"$ref": "acct:A-100"}, "to": {"$ref": "acct:A-200"},
          "amount": {"$decimal": "10000.00"}, "currency": "INR"}}}],
      "intent": {"reason": "customer_requested_transfer", "request_ref": "ticket-88"}
    }"#;

    #[test]
    fn decodes_the_spec_example() {
        let r = TypedJson.decode_request(TRANSFER.as_bytes()).unwrap();
        assert_eq!(r.tx_id.as_str(), "tx-1042");
        let Operation::Named { op, inputs } = &r.ops[0] else {
            panic!("named op")
        };
        assert_eq!(op.to_string(), "transfer@1");
        assert_eq!(
            inputs["amount"],
            Value::Decimal(Decimal::parse("10000.00").unwrap())
        );
        assert_eq!(
            inputs["from"],
            Value::Ref(ObjectId::parse("acct:A-100").unwrap())
        );
        let again = TypedJson
            .decode_request(&TypedJson.encode_request(&r))
            .unwrap();
        assert_eq!(again, r);
    }

    #[test]
    fn error_paths_are_precise() {
        let bad = TRANSFER.replace("\"10000.00\"", "10000.5");
        assert!(
            matches!(TypedJson.decode_request(bad.as_bytes()), Err(DecodeError::NonCanonical(m)) if m.contains("$.ops.0.named.inputs.amount"))
        );
        let bad = TRANSFER.replace("\"$ref\": \"acct:A-200\"", "\"$reff\": \"acct:A-200\"");
        assert_eq!(
            TypedJson.decode_request(bad.as_bytes()),
            Err(DecodeError::UnknownField(
                "$.ops.0.named.inputs.to.$reff".into()
            ))
        );
    }
}
