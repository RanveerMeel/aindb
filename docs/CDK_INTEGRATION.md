# AINDB ↔ CDK Integration Contract

AINDB is a separate project. CDK is optional. The integration lives entirely
in the `aindb-cdk` crate and uses only CDK's published wire formats (the
`cdk-link` crate), never kernel internals.

## Division of responsibility

| Concern | Owner |
|---|---|
| Which agent may perform which operation | **CDK** (capabilities, policy, human approvals) |
| Transactional correctness, data integrity, durable history | **AINDB** |
| Independent checks of proposals and evidence | **Verifier** (via `aindb-verify`) |
| Proposing actions | **Agent** — never grants itself authority |

The database is **not** placed inside the kernel: storage engines, indexes
and transaction processing would enlarge CDK's trusted computing base.

## 1. Authorization: CDK capability → AINDB grant

- CDK object names map to AINDB namespaces: `aindb:<db>/<namespace>`
  (e.g. `aindb:bank/acct`).
- CDK `Permission` values (`src/capability.rs`) map to AINDB op classes:

| CDK permission | AINDB ops |
|---|---|
| `Read` | `get`, `scan`, `history` |
| `Write` | `create`, `update` |
| `Delete` | `delete` |
| `Execute` | named ops (each named op may additionally require a specific grant) |
| `RequiresApproval` | every write in the request returns `NeedsApproval` until a CDK human approval is attached |
| `Write` on `aindb:<db>/_schema` | `define_schema`, named-op registration |

- `aindb-cdk::CdkAuthorizer` implements `aindb_policy::Authorizer`:
  1. Parse the token presented in `PrincipalClaim::Cdk`.
  2. Verify the hybrid signature (Ed25519 **and** ML-DSA-65, both required)
     against the pinned CDK issuer / registry keys.
  3. Check expiry and revocation list (as distributed by the CDK gateway).
  4. Map object + perms to an AINDB grant; deny by default.
- Evidence recorded in the event: CDK issuer id, token hash, node id, perms.
- `NeedsApproval` decisions are not resolved inside AINDB; the agent obtains
  a CDK human-approval and resubmits with the approved token.

## 2. Transport

- M5: AINDB server listens on a local Unix socket; the CDK host gateway
  forwards agent requests as a tool (`tool:aindb`), the same way CDK
  exposes MCP tools today.
- Later: native AINDB service over the CDK PQ link (`CDK-LINK-v1`).

## 3. Audit anchoring

- On each AINDB checkpoint, `aindb-cdk` submits
  `(aindb_node_id, seq, head_hash, checkpoint_sig_hash)` to CDK's audit log
  as a dedicated event kind.
- CDK's signed checkpoints then witness AINDB's history: truncating AINDB
  below an anchored seq is detectable without trusting AINDB's own key.

## 4. Key material

- AINDB's checkpoint key is a hybrid Ed25519 + ML-DSA-65 key, the same
  algorithm pair as CDK, so CDK tooling can verify it.
- In CDK mode the AINDB node key MAY be certified by the CDK registry
  (as CDK node certificates are), binding AINDB checkpoints to an attested
  CDK node.
- Hardware-backed custody (HSM / TPM-sealed keys) is an edition concern and
  is not specified in the public repo.

## 5. Versioning

- Domain separators: `AINDB-OP-v1`, `AINDB-EVENT-v1`, `AINDB-CKPT-v1`,
  `AINDB-ANCHOR-v1`.
- `aindb-cdk` pins a `cdk-link` version; incompatible CDK wire changes bump
  the adapter's major version, never the engine's.
