# fs-mcp – Design (stage 1)

MCP tool generator for the Cap'n Proto contracts (ADR 0021 with the extension of 2026-11-07, ADR 0054 "Lesen ist nicht harmlos",
ADR 0056). **Stage 1 only lists:** it turns the contracts into tools and resource templates *as data*. There is no server, no
bridge and no call. Every later execution must refuse until the engine brake (Thread 2, `fs-messenger-guard`) exists.

**Location is provisional** (crate in `fs-interfaces`, branch `mcp`); the strategy decides the final repo.

## Pattern map (ADR 0043)

| Where | Pattern | Why |
|-------|---------|-----|
| `RiskSource` (`risk.rs`), `MarkFiles` (`marks.rs`) | Strategy | Marks come from files now and from annotations in the contracts later; only a new implementation of the trait is needed, the generator does not change |
| `ContractSource` (`source.rs`), `CapnpcSource` | Strategy + Adapter | the generator reads its own model (`model.rs`); the `CodeGeneratorRequest` of our compiler is translated once, in `capnpc_source/` |
| `ToolGenerator`, `Generator` (`generator/`) | Strategy; inside, Template method (`vet.rs`) | one fixed order of checks for every candidate: block list, mark, exclusion, secret scan |
| `Blocklist` (`blocklist.rs`) | Registry | fixed table target to reason, in code so a mark file cannot lift it |
| `Target` | Newtype | `file/Owner.name` keys for marks, releases and the block list |
| `SchemaBuilder` (`jsonschema.rs`), `tools.rs`, `resources.rs` | Builder | `$defs` collected once per struct; tools assembled from parts |

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

1. **Block list** (`blocklist.rs`, wins over any mark): `MessageAction.decide`, `MessageAction.none`, interface `Guard`,
   `Messenger.guard`, `AccountSetup.secret`. Entries may point at things that do not exist yet (`Guard` is not on `main`).
2. **Marked `excluded`** in `marks/*.toml`; a mark on a *method* holds for all its union members.
3. **Fail-closed secret scan** (`scan.rs`, `secrets.rs`): every field, parameter, result, method and union member name that
   contains a secret word (`secret password passphrase token key credential apikey private seed otp auth cookie`), or whose
   comment says secret/password/passphrase/credential, keeps the tool out **unless a written release exists** in
   `[release]` of the mark file, with a reason. A forgotten mark can therefore never free a secret; it costs a tool.
4. Admin (ADR 0056): marked `excluded` by hand (rights, roles, delegations, memberships and invitations that may confer an
   admin role, storage configuration). There is no machine rule for "admin"; see open points.

## Mark files

`marks/<schema>.toml`: `[Owner]` tables with `method = "class"` or `{ risk, capability, resource, reason }`, and `[release]`.
Strict parsing (unknown class, key or shape is an error). Tests: every method/member of the real schemas has a mark
(`real_contracts.rs`), no mark points at nothing, a new interface/method/union member/secret field in an edited copy of
`messenger.capnp` is listed or excluded (`schema_changes.rs`).

## State

Done: model, traits, schema reader, generator, mark files for all 15 schemas, example `list_tools`. 336 tools, 13 event
resource templates and 2 blob templates from the 15 schemas at this commit.
Not done (on purpose): any server, bridge, call or handle store; the annotation file `schema/mcp.capnp` and the import of it
in the other schemas (new contract, needs the consultation rule, other threads are editing `messenger.capnp`); tool grouping
(problem 12, the messenger alone yields 36 tools, all schemas 336); the output wrapper that keeps hostile message text apart (problem 11).

## Releases written (secret-looking names that are not secrets)

`Reaction.key` (emoji), `Usage.inputTokens`/`outputTokens` (counts), `Draft.encryptionPlan.missingKeys` (addresses),
`Target.hasKey` (bool), `CheckResult.key` (verdict `KeyCheck`). The reason is in `marks/*.toml`.

## What the generator cannot recognise reliably

* **Comments:** the official compiler (and ours, identical on these schemas) attaches a comment block on the lines *after* a
  declaration to that declaration. A comment written *above* the next declaration without a blank line therefore lands on the
  previous one (`common.capnp`: the `Cursor` comment is on `Timestamp`). Descriptions can be a neighbour's. Remedy: blank line
  before a comment block that belongs to the next declaration, or trailing comments.
* **Secrets that carry no secret word** (`proof`, `Data` blobs, a `Text` named `value`, free-form `config` or `json` text)
  are not found; `Backends.add(config)` and `Roles.loadConfig(json)` are excluded by hand for that reason.
* **Admin** has no marker in the contracts; the list in the marks is a judgement, see questions.
* **Capability ids** exist only in comments and several methods share one (`blob.write`), so a 1:1 name is not derivable;
  only `message.send` is given.

## Open points / questions for the strategy

1. Final home of the crate; whether marks stay in `marks/` until `schema/mcp.capnp` exists.
2. A mark on the *method* is ignored when the method is split by a union; only `excluded` propagates. Intended?
3. Streaming key tools (`Streams.create`, `rotateKey`, `addTarget`, `setTargetKey`) are excluded whole. Alternative: offer
   them with the key field removed from the schema (the person reads the key on their own device).
4. Membership/invitation methods that take a `role` are excluded as possible admin conferral; `Memberships.remove`
   is `destructive`. Confirm or give a rule.
5. Provider-facing interfaces (`BlobBackend`, `GroupDirectory.apply/remove`, `Hub.Session`, `Hub.register`) are excluded as
   not a capability of a person. Confirm.
6. Handle expiry and per-call authorisation (problem 5) need the engine; stage 1 only names the handle's interface.
