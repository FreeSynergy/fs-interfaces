# fs-mcp – Design (stage 1)

MCP tool generator for the Cap'n Proto contracts (ADR 0021 with the extension of 2026-11-07, ADR 0054 "Lesen ist nicht harmlos",
ADR 0056). **Stage 1 only lists:** it turns the contracts into tools and resource templates *as data*. There is no server, no
bridge and no call. Every later execution must refuse until the engine brake (Thread 2, `fs-messenger-guard`) exists.

**Location is provisional** (crate in `fs-interfaces`, branch `mcp`); the strategy decides the final repo.
**Marks stay in `marks/*.toml`.** They move into Cap'n Proto annotations after the sprint; there is deliberately no
`schema/mcp.capnp` now, and no other schema imports one. The trait `RiskSource` makes that move local.

## Pattern map (ADR 0043)

| Where | Pattern | Why |
|-------|---------|-----|
| `RiskSource` (`risk.rs`), `MarkFiles` (`marks.rs`) | Strategy | Marks come from files now and from annotations later; only a new implementation of the trait is needed |
| `ContractSource` (`source.rs`), `CapnpcSource` | Strategy + Adapter | the generator reads its own model; the `CodeGeneratorRequest` of our compiler is translated once, in `capnpc_source/` |
| `ToolGenerator`, `Generator` (`generator/run.rs`) | Strategy | the generator takes any `RiskSource` and any contracts |
| `Screen` (`generator/screen.rs`), `vet.rs`, `Blocklist`, `secrets.rs` | Specification | pure predicates over names, comments and types: the candidate is out unless every one holds; the walk inside `screen.rs` is a Visitor with one arm per type variant |
| `Target` | Newtype | `file/Owner.name` keys for marks, releases and the block list |
| `model.rs`, `Tool`, `ResourceTemplate`, `Variant`, `Catalog` | Value object | plain data compared by value |
| `jsonschema.rs`, `capnpc_source/types.rs` | Visitor | one arm per `TypeRef` / Cap'n Proto type variant, outside the data |
| `capnpc_source/names.rs` | Registry | node id to declared name and comment |

## What it does

Per **method** and per **union member** (a method with a pure-union parameter, `Conversation.act`, is split) one tool:
name, description (schema comment), JSON Schema in and out, `_meta["net.freesynergy/mcp"]` with the risk class.
Capabilities are flat tools with a `handle` argument (not for root interfaces). Listeners and `-> stream` give no tool;
`subscribe` becomes an events resource template, methods marked `resource = true` (`Blob`) become blob templates.

Names: the capability id if the mark gives one (`message.send`), else `file.interface.method[.member]`.

## Risk model

`read`, `add`, `change`, `destructive`, `acts-in-name`, `excluded`, `unknown`. **No mark = `unknown` = asks always, like
destructive**, and the target is listed in `Catalog::unmarked`. There is no "harmless": `readOnlyHint` is always `false`,
`destructiveHint` always `true` (hints are untrusted anyway), `_meta.needsRelease` always `true`. Whether a *read object* is
sensitive (ADR 0029) is decided by the engine at the object, never here.

## Exclusions (tests fail if one appears)

Principle (strategy): **what is unclear stays out.** A secret once released cannot be taken back.

1. **Block list** (`blocklist.rs`, wins over any mark): the targets `MessageAction.decide`, `MessageAction.none`,
   interface `Guard`, `Messenger.guard`, `AccountSetup.secret`, and **by type, wherever it appears** (parameter, result,
   field, listener event, any file): `ToolCallDecision`, `GuardProof`, `AiGrant`, `Guard`. Renaming a method lifts nothing.
2. **Marked `excluded`** in `marks/*.toml`; a mark on a *method* holds for all union members it is split into. A member
   without its own mark is `unknown`; it inherits nothing.
3. **Screen** (`screen.rs`) on method, member, every parameter and result, everything reachable from them, **and the
   event types a listener delivers** (`subscribe` becomes a resource only if `Event` and all inside pass):
   * secret words (`secret password passphrase passcode token key credential apikey private seed otp totp oauth auth
     authorization bearer cookie session pin mnemonic signature proof`), split at camelCase and acronyms
     (`IMAPPassword`), in field, parameter, method and **type** names (`LoginCredentials`) and in comments of fields
     and parameters (secret, password, passphrase, credential, token, mnemonic);
   * free-form content: any `Data`, and `Text` named json, config, proof, payload, blob, data;
   * a structure the index does not hold, and `AnyPointer`: **unclear = out**.
   Each is lifted only by a `[release]` entry for exactly that field (`Owner.name`, `Iface.method.param`, or a type name)
   with a reason. `key` is not released anywhere else.
4. Admin (ADR 0056) and provider interfaces: whole interfaces are marked `excluded`, reads included: `Delegations`,
   `Memberships`, groups `Invitations`, `Roles`, files `Backends`, `BlobBackend`, `GroupDirectory`, hub `Session` and
   `Hub.register`, `Items.addParent`. Streaming methods with keys are out whole.
5. `resource = true` is accepted only with class `read` (checked when the marks are read) and only on a method that
   returns `Data` (checked when generating: otherwise an error).

## Mark files

`marks/<schema>.toml`: `[Owner]` tables with `method = "class"` or `{ risk, capability, resource, reason }`, and `[release]`.
Strict parsing (unknown class, key or shape is an error). Tests: every method/member of the real schemas has a mark
(`real_contracts.rs`), no mark points at nothing, a new interface/method/union member/secret field in an edited copy of
`messenger.capnp` is listed or excluded (`schema_changes.rs`).

## State

Done: model, traits, schema reader, generator, mark files for all 15 schemas, example `list_tools`. 307 tools, 13 event
resource templates and 1 blob template from the 15 schemas at this commit.
Not done (on purpose): any server, bridge, call or handle store; the annotation file `schema/mcp.capnp` and the import of it
in the other schemas (new contract, needs the consultation rule, other threads are editing `messenger.capnp`); tool grouping
(problem 12, the messenger alone yields about 36 tools, all schemas 307); the output wrapper that keeps hostile message text apart (problem 11).

## Releases written (each judged by me, to be confirmed)

Not secrets: `Reaction.key` (emoji), `Usage.inputTokens`/`outputTokens` (counts), `Draft.encryptionPlan.missingKeys`
(addresses), `Target.hasKey` (bool), `CheckResult.key` (verdict), `MessageSummary.signature` (verdict enum),
`Conversation.pin` (pins a conversation), `InviteReport.already` (people; the comment mentions a token).
Free-form, released: `Cursor.opaque` (paging position issued by the program; **without it about 27 list/query tools
drop**), `ToolCall.argumentsDigest` (engine digest), `Blob.read.data` (file bytes: the engine decides at the object).
Left out because not released: `Upload.write`, `Draft.attach`, `Boards.import`, `Results.export`, `TileSource.tile`,
`Respond.challenge`, `Credential.proof`.

## What the generator cannot recognise reliably

* **Comments:** the official compiler (and ours, identical on these schemas) attaches a comment block on the lines *after* a
  declaration to that declaration. A comment written *above* the next declaration without a blank line therefore lands on the
  previous one (`common.capnp`: the `Cursor` comment is on `Timestamp`). Descriptions can be a neighbour's. Remedy: blank line
  before a comment block that belongs to the next declaration, or trailing comments.
* **Secrets that carry no secret word** (`proof`, `Data` blobs, a `Text` named `value`, free-form `config` or `json` text)
  are not found; `Backends.add(config)` and `Roles.loadConfig(json)` are excluded by hand for that reason.
* **Admin** has no marker in the contracts; the exclusions are a judgement by interface.
* **Limits of the screen:** it follows structs, lists and groups; a capability (interface) is only a handle and is not
  entered, so what a capability returns is judged on its own methods. Named structures used as parameters are read
  through the type index; a structure in a file that was not compiled is "unclear" and excluded. The word lists
  find names, not meaning: a secret under a harmless name (`value`, `text`) is not found.
* **Capability ids** exist only in comments and several methods share one (`blob.write`), so a 1:1 name is not derivable;
  only `message.send` is given.

## Open points / questions for the strategy

1. Final home of the crate. Marks move into annotations after the sprint.
2. **Size:** 307 tools are a lot for one model (problem 12). Proposal: group by capability and offer a group only after
   the person releases it; stage 1 only lists.
3. Releases above are my judgement; `Cursor.opaque` and `Blob.read.data` matter most.
4. Handle expiry and per-call authorisation (problem 5) need the engine; stage 1 only names the handle's interface.
