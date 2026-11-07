# fs-interfaces – Design

Public home of the interface contracts (ADR 0045, 0050). It holds **no program logic**: schemas, their generated Rust code,
and the Rust side of the provider rule. Each schema has exactly one owner program, named in its header; the owner decides
changes, consumers depend on this repo by git tag (ADR 0043, `coding.md` §8).

## Structure and pattern map (ADR 0043)

| Part | Pattern | Why |
|---|---|---|
| `schema/*.capnp` | Interface / Specification | the contracts; draft until two consumers, then 1.0, then only jumps |
| `src/*_capnp.rs` (generated, committed) | Adapter | wire form; one module per schema, no generator needed at build time |
| `src/provider.rs` `Unsupported` | Null Object | the explicit "not possible" (RFC 9457 `Problem`, status 501) |
| `src/provider.rs` `Capabilities` | Value object | a provider's report of what it can do (`Support` per capability) |
| `src/rpc/*` (feature `rpc`) | Adapter, Factory, Observer; `rpc/problem`: Value object (`Problem`, extensions), Adapter (`wire`: error text), `keys`: `Pattern: none`, constants | the shared server and client side of every contract: socket, RPC system, envelope, RFC 9457 problems as RPC errors, pages, change feed |
| `src/rpc/client/*` (feature `rpc`) | Command (+ Strategy `Remote`, RAII guard `Reply`); `Timeouts`/`Fault`: `Pattern: none` in the file, plain data types; `problems`: `Pattern: none`, free helper functions | the client half every program's client shares: the thread that owns the `!Send` connection, fed by commands from any thread (the command is a boxed closure, not an enum with `apply`: the calls differ per contract); the guard turns a command that ends without an answer into `Fault::Interrupted` instead of "connection gone"; deadlines as configuration; `Problem` read back typed. Mapping to a program's own `Error` and its problem type URIs stay in that program's client |
| `crates/regenerate` | Pipeline | `capnp-compiler` -> `CodeGeneratorRequest` -> `capnpc` -> files |
| `tests/generated_matches_official.rs` | Oracle test | official compiler and ours must produce the committed code |

## State

Version 0.43.0 (`fs-store` v0.4.0, so a hub on 0.4 builds with one `Envelope` type; `SupportReport.instance @3` (the store's `StoreId`, empty = not known); `events.Items.occurrences` answers `damaged @1` (struct `Damaged`: id, reason) and `cut @2` (items cut short at a bound, no error); `people` comments say "full IRI" for roles; append only). Version 0.42.0 added the workspace crate `fs-mcp`, the MCP tool generator stage 1 (see `crates/fs-mcp/DESIGN.md`).

| Area | State |
|---|---|
| `common.capnp` (draft 0.2): `Uuid`, `Timestamp`, `Cursor`, `Problem`, `Blob`, `Envelope` (incl. `describes`, `startTime`, `endTime`), `CapabilityList`, `Change`, `ChangeListener`, `Subscription` | done |
| Provider rule types: `Support`, `Unsupported`, `SupportReport`, const `unsupportedProblemType` | done, draft |
| `tasks.capnp` (from fs-tasks), `hub.capnp` (from fs-hub), `messenger.capnp` (from fs-documentation `temp/contracts`) | moved, all draft; the three copies of `common.capnp` were identical |
| `people.capnp` (new, owner fs-people): people, relations, same-person links, role definitions and assignments, AI marking | done, draft, provider: fs-people-core; no consumer yet |
| `groups.capnp`, `files.capnp` (new) | done, draft; `files` is provisional, no file frontends analysed yet |
| `events`, `places`, `texts` (new); `messenger` gains `senderKind`, `toolCalls`, `taskRefs`; `tasks` gains `supportReport` | done, draft |
| Corrections from the first providers: `files` gains `moveFolder`, `pending`, `FileFilter.trashed`; capability ids of `tasks` written down; rules of `groups` stated | done |
| `texts` gains `moveFolder` (the text at a path and everything below it move; a folder without a text has no id) | done, version 0.6.0 |
| `groups` gains `RightFlow` (none, down, up), `Group.rightFlow @4`, `Items.setRightFlow @12` (ADR 0055 point 6) | done, version 0.7.0 |
| `files` gains `Folders.rescan @13`, `RescanReport`, three named problem types, and `BlobBackend` naming/publish/relocate/discard/makeDir/removeDir/scan; `envelope.name` is the real name | done |
| `files` gains `OnConflict` (refuse, keepBoth) as a last parameter of rename, move, copy (file, folder where it applies) and `startUpload`; no replace | done |
| `files` gains `Sort`/`SortBy` as a last parameter of `Folders.list` and `Search.find` | done |
| `files` gains size and date filters in `FileFilter` (@4 to @9) | done |
| `files` gains `Folders.emptyTrash @15`, `restoreTo @14`, `TrashReport`, and `onConflict` on `restore` | done |
| `files` gains `Folders.getFolder @16` | done |
| `files` gains `Transfer.pendingUploads @3` and `UploadInfo` | done |
| `groups` gains parent consent (`Items.requestParent @13`, `approveParent`, `declineParent`, `parentRequests`, `ParentRequest`, `RequestState`; `addParent @6` superseded), `Invitations.acceptProved @5`, `GroupDirectory.directoryId @5` (all provisional) | done, version 0.19.0 |
| `groups` gains `inForce @4` on `Membership` and `Delegation` (ADR 0056: false = on record but not counted; an unaware reader sees false) | done, version 0.31.0 |
| `surveys.capnp` (new, owner fs-surveys): surveys, anonymous ballots released in batches, methods, widening, decisions linked to tasks | done, draft, provisional (no frontends analysed) |
| `messenger` (draft 0.2, ACP and MCP analysis, ADR 0054; ordinal 0 of `Encryption`, `MessageAction`, `ToolCallKind` renumbered to a harmless value): `ToolCall` gains `risk`, `outcome`, `detail`, `result`, `options`, `argumentsDigest`, `askedOf`, `revision`; `ToolCallDecision` with `signer`; `MessageAction.decide`; events `toolCall` and `text`; `Message.reasoning`, `bodyMediaType`; `MessageSummary.jobRef`; `Conversation.stop`, `usage`; `Messenger.supportReport` | done, draft |
| `history.capnp` (new, owner fs-history): entries, filter, digest, forget | done, draft, provisional |
| `resources.capnp` (new, owner fs-resources): things, reservations, capacity owned by the person, needs and offers | done, draft, provisional |
| `whiteboard.capnp` (new, owner fs-whiteboard): boards, elements, merge per property, undo per editor | done, draft, provisional |
| `messenger` draft 0.3: `Draft.setSubject`; `MessageSummary.arrival`, `undecrypted`, `signature`, `tooLarge`; `skipped` on both pages; `unknown` as ordinal 0 of `Kind`, `ToolCallState`, `DeliveryState`; revision rule stated in full | done, draft |
| Contracts round 2 (draft, append only, strategy order): `texts` `revision` comment corrected (changes at every write, equality only), `LinkRow.title`/`internal`; `files` `pendingUploads` and `BlobBackend.scan` take a `Cursor` and answer `next`, `TrashReport.failedMissing` and the bound of `failed`, seven more problem types named (not-found, conflict, wrong-offset, invalid, encryption-required, stored-data-invalid, backend-failed); `revision` on `Event`, `Person`, `Task` and `events.Items.updateIf @9`, `people.Persons.updateIf @23`, `tasks.Items.editIf @12` (compare-and-set as `texts.editIf`, reason `conflict:`). `pendingUploads`/`scan` keep their ordinals and grow a parameter and a result field: on the wire not incompatible, but a 0.33 client gets only the first page from a 0.35 server (draft, no caller over RPC yet). The `failed` bound uses 24 bytes per id as a deliberate margin (a `Uuid` is 16 on the wire) | done, version 0.35.0 |
| `people` `RoleDefinition` gains `type @8`, `broader @9`, `closeMatch @10`, `standardRefs @11` (SKOS roles, `roles.md`; append only, empty `type` = `skos:Concept`) | done, version 0.36.0 |
| `rpc` `Problem` gains `extensions` (RFC 9457 extension members, typed: `with_ext`, `with_ext_u64`, `ext`, `ext_u64`, `ext_all`; at most 64 entries, keys of 64 bytes, all texts 64 KiB, an entry that does not fit is left out whole, never cut; in the error text as `ext<RS>key<RS>value` fields after `next`, a 0.36 reader loses only the cursor of a text that has both, so the `too-large` of a page with a cursor carries none); standard keys `id`, `limit`, `size`, `issue`, `kind` in `rpc::problem_keys`; `too_large_by` and the page checks fill `size`/`limit`; `common.capnp` gains `Extension` and `Problem.extensions @4` (`Problem::write`/`read` fill and read it; the error text carries it as well); `rpc/problem` split into `value`, `wire`, `ext`, `keys`, `answers` | done, version 0.39.0 |
| `files` gains `BlobInfo.modified`, `BlobBackend.has @14`, `sweepPending @15`; `commit` never replaces on a real-names backend | done |
| `streaming.capnp` (new, owner fs-streaming): stage 1: streams, targets, keys as secrets, check, recording handed over | done, draft, provisional |
| `rpc`: an item too large for any message no longer blocks what is behind it (`Problem::next`), error texts are cut at 64 KiB, `run_until_signal` waits for `run_to_end` work, `prepare_data_dir` answers the canonical path, `--instance` must be a domain | done, version 0.32.0 |
| `files` gains `BlobBackend.inspect @16` | done |
| Generated Rust with `capnp-compiler` v0.1.0 + `capnpc` 0.27 | done; byte-identical to `capnpc-embedded` 0.2 output (tested) |
| Rust helpers `Unsupported`, `Capabilities` with wire round trips | done |
| `rpc` (feature): socket with lock file, connection, problems, envelope, pages, feed | done, draft |
| `rpc::client` (feature): `Worker<R: Remote>` (thread, current-thread runtime + `LocalSet`, command queue, `Timeouts` 5 s connect / 30 s call, drop cuts the connection first (commands under way get `Disconnected`, a write may be left half done), then joins; a command that waits on nothing from the connection keeps the drop waiting up to the call deadline; on the connection thread itself nothing is joined), reply guard (`Fault::Interrupted`), `Fault` naming the deadline, `problems` (`Failure::of`, `local_type`, `first_number`, `object_id_in`, `without_store_prefix`, `unsupported_of`); taken from the people and tasks clients, which map to their own `Error` and problem type URIs themselves | done, draft; tests in `tests/rpc_client*.rs` against a real server |
| Tests | 2 oracle, 3 provider, 40 `rpc` (+21 problem extensions and text format), 19 `rpc::client` (+2 unit), 1 doc test |
| Interfaces at 1.0 | none; the draft -> 1.0 gate needs two independent consumers per interface |

## Decisions

* `capnp` 0.27 for all consumers (fs-hub moved from 0.23).
* Generated code is not formatted (`#[rustfmt::skip]`) and exempt from lints; `regenerate` is a workspace member, not a feature,
  so it works before the library compiles.
* `rpc` lives here, not in a repo of its own, because generated types of two `fs-interfaces` tags are different Rust types: a helper crate would have to be re-tagged for each contract tag. Behind a feature, so a program that only uses the contracts pulls in no runtime. Provisional, reported to the strategy session.
* A feed never ends silently: `ChangeListener.ended @1` tells the consumer, who subscribes again and then reads the current state (strategy session, 2026-11-06).
* A problem crosses the wire inside the exception text behind a marker (`rpc::Problem`), because the contract methods have no error result; status 501 also sets the exception kind `unimplemented`. 
* The Rust trait with `Unsupported` defaults for each topic lives with the owner program (ADR 0043: trait first); this repo
  holds only the shared types.

* `supportReport` is the standard way to ask what a provider offers (strategy session, 2026-11-06). `capabilities` is not added to further contracts and goes before 1.0; ADR 0021 stays met, the report is what API, CLI and MCP query. The hub's `Manifest.capabilities` is another thing, what a program declares, and stays.

## Limits of `rpc`, to close before 1.0

* **Pages:** the cursor is opaque to consumers (a leading version byte, nothing to calculate with). Today it marks a place in the core's ordered list, so an insert or delete between two pages can skip or repeat one entry. Before 1.0 it becomes a stable key (sort value plus id) without a contract change. For messages and `subscribe`, skipped or repeated entries count as errors (strategy session, 2026-11-06).
* **Time:** `Timestamp` carries milliseconds and the store microseconds, so two updates within one millisecond look alike to a consumer comparing `updated`. Needs a contract decision.
* **Message size:** one message above 64 MiB ends the connection with all its capabilities. Servers cut answers: pages at 32 MiB (`Page::of_weighted`), reads (`clamp_read`), error texts at 64 KiB (`MAX_DETAIL_BYTES`). A single item above 32 MiB comes alone on its page, up to 63 MiB (`MAX_ITEM_BYTES`, a message less 1 MiB). Above that it cannot be sent: the call answers `too-large` with a cursor past it in the error text (`Problem::next`, stated on `Cursor` in `common.capnp`), so a consumer can skip on. A client that sends more than a message loses its connection.
* **Read limits:** `serve_with` and `connect_with` stay, because `fs-hub` will use them. They set what that end reads; `Page`, `check_answer`, `check_intake` and `clamp_read` stay capped at the default.
* **Stopping:** after a signal `run_until_signal` waits up to 30 s (`FINISH_TIMEOUT`) for `run_to_end` work and then exits with an error naming what is left. It keeps the socket and the lock during the wait. Other method calls are still cancelled, and new `run_to_end` work is refused as `disconnected`.

## What a program now refuses at the start

`bind_socket` and `prepare_data_dir` refuse to start (`PermissionDenied`) when an ancestor of the socket or data directory is group- or world-writable and not sticky, or is owned by another user who is not root. Someone else could swap the directory otherwise. This hits setups that used to run:

* a `$HOME` of 0775, as some systems create it for user-private groups; the default data directory lies below it;
* a project directory shared with a group (0775 or 2775) holding `--data` or `--socket`;
* a directory below the home of another user, or on a mount owned by a service user.

The remedy is `chmod g-w` on the directory the message names, or another `--data` or `--socket`. Sticky directories such as `/tmp` stay allowed.

## Tags not to use

`fs-interfaces` v0.18.0 to v0.31.0 for `rpc`, superseded because of these findings: an oversized item makes all behind it unreadable, or an item above a message ends the connection; an error text has no size limit, so one bad request ends the connection; a signal cancels writes under way. Their contracts are fine; only the feature `rpc` should not be used.

## Open points

* Owner of `common.capnp` (`fs-store` is proposed in ADR 0050).
* More topics from the catalogue (people, groups, events, files ...) as their owners write drafts.

## fs-mcp

The MCP tool generator (stage 1, lists only) lives in `crates/fs-mcp`; see its `DESIGN.md`.
