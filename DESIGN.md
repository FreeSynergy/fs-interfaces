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
| `crates/regenerate` | Pipeline | `capnp-compiler` -> `CodeGeneratorRequest` -> `capnpc` -> files |
| `tests/generated_matches_official.rs` | Oracle test | official compiler and ours must produce the committed code |

## State

Version 0.16.0.

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
| `surveys.capnp` (new, owner fs-surveys): surveys, anonymous ballots released in batches, methods, widening, decisions linked to tasks | done, draft, provisional (no frontends analysed) |
| `messenger` (draft 0.2, ACP and MCP analysis, ADR 0054; ordinal 0 of `Encryption`, `MessageAction`, `ToolCallKind` renumbered to a harmless value): `ToolCall` gains `risk`, `outcome`, `detail`, `result`, `options`, `argumentsDigest`, `askedOf`, `revision`; `ToolCallDecision` with `signer`; `MessageAction.decide`; events `toolCall` and `text`; `Message.reasoning`, `bodyMediaType`; `MessageSummary.jobRef`; `Conversation.stop`, `usage`; `Messenger.supportReport` | done, draft |
| Generated Rust with `capnp-compiler` v0.1.0 + `capnpc` 0.27 | done; byte-identical to `capnpc-embedded` 0.2 output (tested) |
| Rust helpers `Unsupported`, `Capabilities` with wire round trips | done |
| Tests | 2 oracle, 3 provider, 1 doc test |
| Interfaces at 1.0 | none; the draft -> 1.0 gate needs two independent consumers per interface |

## Decisions

* `capnp` 0.27 for all consumers (fs-hub moved from 0.23).
* Generated code is not formatted (`#[rustfmt::skip]`) and exempt from lints; `regenerate` is a workspace member, not a feature,
  so it works before the library compiles.
* The Rust trait with `Unsupported` defaults for each topic lives with the owner program (ADR 0043: trait first); this repo
  holds only the shared types.

## Open points

* Owner of `common.capnp` (`fs-store` is proposed in ADR 0050).
* More topics from the catalogue (people, groups, events, files ...) as their owners write drafts.
