# AINDB-OP-v1 — Typed (JSON) Form

Reference for the typed encoding implemented in `aindb_protocol::TypedJson`
(ADR-0004). The binary encoding (M4) carries the same structure.

## Strict JSON

Input is UTF-8 JSON (RFC 8259) with extra rules. Breaking any rule rejects the
whole request before anything executes.

| Rule | Error |
|---|---|
| No floating point or exponents (`1.5`, `1e3`) — use `{"$decimal": "1.50"}` | `NonCanonical` |
| Integers canonical: no leading zeros, no `-0` | `NonCanonical` |
| No duplicate object keys | `DuplicateKey` |
| No unknown fields anywhere | `UnknownField` |
| Nothing after the request but whitespace | `TrailingBytes` |
| No lone surrogates or raw control characters in strings | `Invalid` |
| Nesting depth ≤ 64, request ≤ 8 MiB, ≤ 256 ops, ≤ 256 preconditions | `LimitExceeded` |
| Object state / patch / named-op inputs ≤ 1 MiB (canonical encoding) | `LimitExceeded` |
| Scan and history `limit` 1..=10 000 | `LimitExceeded` |

Error messages carry a path such as `$.ops.0.create.state.balance`.
`LimitExceeded` maps to error code `limit_exceeded`; every other decode
error maps to `malformed`.

## Identifiers

| Kind | Form | Example |
|---|---|---|
| Object id | `ns:key`; `ns` = `[a-z0-9_-]{1,32}`, key = `[A-Za-z0-9._-]+`, ≤ 255 bytes | `acct:A-100` |
| Tx id | `[A-Za-z0-9._:-]{1,64}`, chosen by the agent (idempotency key) | `tx-1042` |
| Schema / named op | `name@major`, name = `[a-z][a-z0-9_]*`, major ≥ 1 | `account@1`, `transfer@1` |

The namespace is the unit of authorization.

## Values

| Type | JSON |
|---|---|
| null | `null` |
| bool | `true` / `false` |
| i64 | integer |
| text | string |
| list | array |
| map | object (keys must not start with `$`) |
| u64 | `{"$u64": 18446744073709551615}` |
| decimal | `{"$decimal": "10000.00"}` — fractional digits = scale (0–18), no `+`, no `-0` |
| bytes | `{"$bytes": "00ff"}` — lowercase hex |
| timestamp | `{"$timestamp": 1700000000000000000}` — UTC ns since epoch |
| ref | `{"$ref": "acct:A-100"}` |

Field types in schemas: `bool`, `i64`, `u64`, `decimal(N)`, `text`, `bytes`,
`timestamp`, `ref(schema@N)`, `list(T)`, `map(T)`. A decimal must have exactly
the field's scale: `"10.0"` is rejected for `decimal(2)`.

## Request

```json
{
  "protocol": "AINDB-OP-v1",
  "tx_id": "tx-1042",
  "principal": {"api_key": "…"},
  "ops": [ … ],
  "preconditions": [ … ],
  "intent": {"reason": "customer_requested_transfer", "request_ref": "ticket-88"},
  "options": {"dry_run": false, "want_explanation": true}
}
```

`preconditions`, `intent`, `options` and `intent.request_ref` are optional.

**principal** — exactly one of:
`{"api_key": "…"}` · `{"ed25519": {"key_id": "…", "signature": "<hex>"}}` ·
`{"cdk": "<hex token>"}`

**preconditions** — each exactly one of:
`{"exists": "acct:A"}` · `{"not_exists": "acct:A"}` ·
`{"version_is": {"id": "acct:A", "version": 7}}`

## Operations

Each op is an object with exactly one key:

```json
{"create": {"schema": "account@1", "id": "acct:A-100", "state": {"owner": "asha", "balance": {"$decimal": "0.00"}}}}
{"update": {"id": "acct:A-100", "expect_version": 3, "patch": {"owner": "asha k"}}}
{"delete": {"id": "acct:A-100", "expect_version": 4}}
{"get":    {"id": "acct:A-100", "at": 120}}
{"scan":   {"index": "by_owner", "start": ["a"], "end": ["b"], "limit": 100}}
{"history":{"selector": {"object": "acct:A-100"}, "from": 0, "limit": 50}}
{"named":  {"op": "transfer@1", "inputs": {"from": {"$ref": "acct:A-100"}, "to": {"$ref": "acct:A-200"}, "amount": {"$decimal": "10000.00"}}}}
{"define_schema": {
   "id": "account@1",
   "fields": [
     {"name": "owner",    "type": "text",       "required": true},
     {"name": "balance",  "type": "decimal(2)", "required": true},
     {"name": "currency", "type": "text",       "required": true, "default": "INR"},
     {"name": "pan",      "type": "text",       "sensitivity": "secret"}
   ],
   "indexes":    [{"name": "by_owner", "fields": ["owner"], "unique": false}],
   "invariants": [{"name": "non_negative", "expr": "balance >= 0"}]
}}
```

Optional: `expect_version`, `at`, `history.from` (default 0), field
`required` (default false), `default`, `sensitivity` (`public` | `internal` |
`secret`, default public), `indexes`, `invariants`, index `unique`.
History selectors: `{"object": id}` · `{"tx": tx_id}` · `{"principal": id}`.

In a `patch`, `null` removes an optional field.

## Result

```json
{
  "tx_id": "tx-1042",
  "status": "committed",
  "reads":  [{"id": "acct:A-100", "version": 3}],
  "writes": [{"id": "acct:A-100", "version": 4}, {"id": "acct:A-200", "version": 9}],
  "values": [],
  "error":   {"code": "invariant_violated", "name": "non_negative"},
  "receipt": {"seq": 1201, "event_hash": "<hex>", "checkpoint_seq": 1024},
  "explanation": "…"
}
```

`status`: `committed` | `rejected` | `aborted` | `dry_run_ok`. `error`,
`receipt` and `explanation` appear only when present. Error codes:
`malformed` (+`detail`), `unauthorized`, `needs_approval`, `schema_violation`
(+`field_path`), `precondition_failed`, `invariant_violated` (+`name`),
`conflict`, `verifier_rejected` (+`verifier`), `limit_exceeded`, `internal`.

## Encoding

The encoder writes compact JSON, map keys in sorted order, and omits optional
members that are absent or default. `decode(encode(r)) == r` and re-encoding
is byte-stable (AT-PROT-01).
