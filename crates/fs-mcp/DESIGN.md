# fs-mcp – Design (stage 1)

MCP tool generator for the Cap'n Proto contracts (ADR 0021 with the extension of 2026-11-07, ADR 0054 "Lesen ist nicht harmlos",
ADR 0056). **Stage 1 only lists:** it turns the contracts into tools and resource templates *as data*. There is no server, no
bridge and no call. Every later execution must refuse until the engine brake (Thread 2, `fs-messenger-guard`) exists.

**Location is provisional** (crate in `fs-interfaces`, branch `mcp`); the strategy decides the final repo.
**Marks stay in `marks/*.toml`.** They move into Cap'n Proto annotations after the sprint; there is deliberately no
`schema/mcp.capnp` now, and no other schema imports one. The trait `RiskSource` makes that move local.

**Eine Liste ist keine Erlaubnis.** The tool list is only an offer. Without an expressly granted authority an AI reads
and does nothing (project owner, ADR 0054); the engine enforces that on every call, the generator does not change for it.
Every tool and resource keeps `_meta.needsRelease: true`. The owner confirmed the admin exclusions (delegations, roles,
memberships, invitations, backends, hub registration: no tool, not even for reading).

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
| `capnpc_source/names.rs` | none | id lookup tables built once per request; nothing is looked up by kind |

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
3. **Screen** (`screen.rs`) on method, member, **a union parameter as a whole before it is split** (parameter name and
   comment, name and comment of the union, block list on its type; the members are then judged on their own), every parameter and result, everything reachable from them, **and the
   event types a listener delivers** (`subscribe` becomes a resource only if `Event` and all inside pass):
   * secret words (`secret password passphrase passcode token key credential apikey private seed otp totp oauth auth
     authorization bearer cookie session pin mnemonic signature proof`), split at camelCase and acronyms
     (`IMAPPassword`), in field, parameter, method and **type** names (`LoginCredentials`) and in comments of fields
     and parameters (secret, password, passphrase, credential, token, mnemonic);
   * free-form content: any `Data`, and `Text` named json, config, proof, payload, blob, data;
   * a structure the index does not hold, and `AnyPointer`: **unclear = out**;
   * type names are compared on every part of a nested path (`Wrap.ToolCallDecision`); comments of structs and
     interfaces are read too; a capability type is judged by its name, comment, block list and interface mark (its
     methods are judged when their own tools are made, not through the handle);
   * a listener is judged like any interface: block list, interface mark (`"@interface"`), name and comment;
   * words are matched exactly and also as run-together lower-case substrings (`accesstoken`, `authtoken`, `pwd`).
   Each is lifted only by a `[release]` entry for exactly that field (`Owner.name`, `Iface.method.param`, or a type name)
   with a reason. `key` is not released anywhere else.
4. Admin (ADR 0056) and provider interfaces are excluded **as interfaces**: `"@interface" = { risk = "excluded" }` in the
   mark file holds for every present and future method, no marks are demanded inside, and **every method that hands out
   a handle to it is excluded too** (`people.people.roles`, `groups.groups.memberships`, ...). Marked so: `Delegations`,
   `Memberships`, groups `Invitations`, `Roles`, `Backends`, `Sharing`, `BlobBackend`, `GroupDirectory`, hub `Session`,
   `TileSource`. Single methods: `Hub.register`, `Items.addParent`/`removeParent`/`approveParent`, `BoardEditor.add`
   (takes a whole `Envelope` with sensitive/to/bcc, ADR 0029). Streaming methods with keys are out whole.
5. A **listener** is an interface whose name ends with `Listener` and which a method takes as a parameter. Any other
   capability passed in is an ordinary handle argument. An events resource needs a mark of class `read`; two equal URIs
   are an error.
6. `resource = true` is accepted only with class `read` (checked when the marks are read) and only on a method that
   returns `Data` (checked when generating: otherwise an error).

## Mark files

`marks/<schema>.toml`: `[Owner]` tables with `method = "class"` or `{ risk, capability, resource, reason }`, and `[release]`.
Strict parsing (unknown class, key or shape is an error). Tests: every method/member of the real schemas has a mark
(`real_contracts.rs`), no mark points at nothing, a new interface/method/union member/secret field in an edited copy of
`messenger.capnp` is listed or excluded (`schema_changes.rs`).

## State

Done: model, traits, schema reader, generator, mark files for all 15 schemas, example `list_tools`. 289 tools, 12 event
resource templates and 1 blob template from the 15 schemas at this commit.
Not done (on purpose): any server, bridge, call or handle store; the annotation file `schema/mcp.capnp` and the import of it
in the other schemas (new contract, needs the consultation rule, other threads are editing `messenger.capnp`); tool grouping
(problem 12, the messenger alone yields about 36 tools, all schemas 289); the output wrapper that keeps hostile message text apart (problem 11).

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

## More limits, stated plainly

* **A comment above a `struct` or `interface` almost never arrives** (it is attached to the previous declaration), so the
  check of struct and interface comments rarely fires. Trailing comments on a line are read reliably.
* **`extends` is not read.** An interface that extends another does not inherit the other's marks or screening.
* A root is an interface that no result, struct field or listener event hands out; the rest need a `handle` argument.
* **Boundary (named, by the strategy): these tools can carry secrets that somebody wrote into specialist text (notes,
  links, URLs with passcodes, mail text). The protection lies with the engine alone: without authority, nothing;
  anything marked sensitive asks every time.** The list is machine-readable: `_meta.freeContent` of every tool
  (`{field, format}`) and `examples/list_free_content.rs`. Every free field has a `[format]` mark in `marks/*.toml`
  ("a format, not a secret field"); a free field without one keeps its tool out, and a test fails if any field of the
  contracts named jscontact, jscalendar, geojson, as2, document, details, markdown, notes, comment, bodyText,
  bodyHtml, reasoning, sieve or html lacks it. **94 tools carry such a field** (89 earlier, plus bodyText/bodyHtml/
  reasoning, sieve, markdown, notes, comment). Links and virtual locations of a JSCalendar event and notes and links of a
  JSContact card live inside those JSON fields. Event resources carry such fields too (messages) and are not counted.
  Not covered by name: `preview`, `text` (TextChunk, import/export), `Extension.value`.
* `Rules.set` is excluded: a Sieve script can forward all mail silently. `Rules.list` stays (read).
* **Free content is judged by name only.** Fields named `jscontact`, `jscalendar`, `geojson`, `as2`, `document`, `details`
  carry whole objects and are not stopped (they can hold anything the person wrote). 89 of the tools carry one
  (events 9, groups 5, people 16, places 6, resources 5, surveys 21, tasks 10, texts 10, whiteboard 7); whether they stay
  offered is for the strategy to decide. Every one still needs a release by the person (`needsRelease`).
* `Target.url` is **not** released: an RTMP key often sits in the path (`rtmp://host/live2/<key>`) and the contract cannot
  tell application from application plus key. `Streams.targets`, `editTarget`, `setTargetEnabled` and the streaming events
  resource therefore stay out. `Stream.ingestUrl` is released (made by the program, key kept apart as `publisherKey`).
* A mark on a union member or a struct field has an effect only through the first pure-union parameter of a method
  (`variant.rs`) and the screen walk; a mark placed anywhere else is silently without effect. Not fixed.
* A listener inside a union or struct, or one whose name lacks `Listener`, is treated as a handle input.
* The reader leaves out a method whose parameters or results are a **named structure with a group or a union**
  (`m @0 Foo -> Bar`): it is listed as unclear and gets no tool.
* `Groups.Follows` (delivering rights between groups) is excluded as an interface.

## Open points / questions for the strategy

0. **Invitations are uneven:** `events.invitations.invite/respond` are tools (`respond` takes a free `who :Text`),
   `surveys.designer.invite` and groups invitations are excluded, and `messenger.accounts.remove` is only `destructive`.
   One rule for all is needed.

1. Final home of the crate. Marks move into annotations after the sprint.
2. **Size:** 289 tools are a lot for one model (problem 12). Proposal: group by capability and offer a group only after
   the person releases it; stage 1 only lists.
3. A release stays valid when a comment changes. Binding it to a hash of type plus comment is possible and not done;
   until then a changed comment on a released field needs a manual look (noted, not built).
4. **Contract question:** `surveys.respond.submit.person` takes a person IRI as the credential of a named answer: the
   caller says who answers. Should the engine fill it from the caller, never from arguments?
5. The compiler attaches a comment to the previous declaration; several false alarms (Streams) are releases for that
   reason. A blank line before a comment that belongs to the next declaration would remove them.
6. `surveys.designer.invite` (an invitation to answer) is excluded for now: no tool for invitations until the strategy
   decides. Question noted.
7. Handle expiry and per-call authorisation (problem 5) need the engine; stage 1 only names the handle's interface.
