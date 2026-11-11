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
| `src/rpc/client/*` (feature `rpc`) | Command (+ Strategy `Remote`, RAII guard `Reply`); `Timeouts`/`Fault`: `Pattern: none` in the file, plain data types; `call_limited` (Decorator, one function): the call deadline for a bare `Connection` too, and the one the `Worker` uses (`Worker::call_within` takes a deadline per call); `problems`: `Pattern: none`, free helper functions | the client half every program's client shares: the thread that owns the `!Send` connection, fed by commands from any thread (the command is a boxed closure, not an enum with `apply`: the calls differ per contract); the guard turns a command that ends without an answer into `Fault::Interrupted` instead of "connection gone"; deadlines as configuration; `Problem` read back typed. Mapping to a program's own `Error` and its problem type URIs stay in that program's client |
| `crates/regenerate` | Pipeline | `capnp-compiler` -> `CodeGeneratorRequest` -> `capnpc` -> files |
| `tests/generated_matches_official.rs` | Oracle test | official compiler and ours must produce the committed code |

## State

Latest tag v0.47.0 (header corrected 2026-11-08; the next lines list 0.44.0 and earlier: `tasks.TaskFilter` gains `term @4` (substring of title and description, case-insensitive, like `PersonFilter.term`) and `open @5` (only needs-action and in-process tasks); append only). Version 0.43.0 (`fs-objects` v0.4.0: the break hit programs with feature `rpc` on fs-objects v0.2.0 (people, tasks, calendar-server, files, wiki), which could not build with fs-objects 0.4 (two `Envelope` types); groups, maps and the hub use fs-interfaces without `rpc` and are not affected; `SupportReport.instance @3` (the store's `StoreId`, empty = not known); `events.Items.occurrences` answers `damaged @1` (struct `Damaged`: id, reason, key) and `cut @2` (items cut short at a bound, no error); `people` comments say "full IRI" for roles; append only). Version 0.42.0 added the workspace crate `fs-mcp`, the MCP tool generator stage 1 (see `crates/fs-mcp/DESIGN.md`).

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
| `messenger` (0.47.0): `ToolCallRisk.readsSensitive` (ordinal 5, last): a read of an object marked sensitive; asks every time like `destroys`, never released | done, draft |
| `store` (0.48.0): FreeSynergy.Store contract (sources, available, installed, install, update, job, subscribe) from the sketch in `fs-store/DESIGN.md`; owner `fs-store`; 1 of 2 consumers (CLI; GUI to come) | new, draft |
| `rpc::cursor` (0.49.0, fixed test 0.49.1): `PlaceParts`, `encode_place`, `decode_place`: the cursor of a page that continues after a place in a sort order; moved here from `fs-tasks-server` and `fs-wiki-server`, which had the same file | new, Rust only |
| Contracts round 2 (draft, append only, strategy order): `texts` `revision` comment corrected (changes at every write, equality only), `LinkRow.title`/`internal`; `files` `pendingUploads` and `BlobBackend.scan` take a `Cursor` and answer `next`, `TrashReport.failedMissing` and the bound of `failed`, seven more problem types named (not-found, conflict, wrong-offset, invalid, encryption-required, stored-data-invalid, backend-failed); `revision` on `Event`, `Person`, `Task` and `events.Items.updateIf @9`, `people.Persons.updateIf @23`, `tasks.Items.editIf @12` (compare-and-set as `texts.editIf`, reason `conflict:`). `pendingUploads`/`scan` keep their ordinals and grow a parameter and a result field: on the wire not incompatible, but a 0.33 client gets only the first page from a 0.35 server (draft, no caller over RPC yet). The `failed` bound uses 24 bytes per id as a deliberate margin (a `Uuid` is 16 on the wire) | done, version 0.35.0 |
| `people` `RoleDefinition` gains `type @8`, `broader @9`, `closeMatch @10`, `standardRefs @11` (SKOS roles, `roles.md`; append only, empty `type` = `skos:Concept`) | done, version 0.36.0 |
| `rpc` `Problem` gains `extensions` (RFC 9457 extension members, typed: `with_ext`, `with_ext_u64`, `ext`, `ext_u64`, `ext_all`; at most 64 entries, keys of 64 bytes, all texts 64 KiB, an entry that does not fit is left out whole, never cut; in the error text as `ext<RS>key<RS>value` fields after `next`, a 0.36 reader loses only the cursor of a text that has both, so the `too-large` of a page with a cursor carries none); standard keys `id`, `limit`, `size`, `issue`, `kind` in `rpc::problem_keys`; `too_large_by` and the page checks fill `size`/`limit`; `common.capnp` gains `Extension` and `Problem.extensions @4` (`Problem::write`/`read` fill and read it; the error text carries it as well); `rpc/problem` split into `value`, `wire`, `ext`, `keys`, `answers` | done, version 0.39.0 |
| `files` gains `BlobInfo.modified`, `BlobBackend.has @14`, `sweepPending @15`; `commit` never replaces on a real-names backend | done |
| `streaming.capnp` (new, owner fs-streaming): stage 1: streams, targets, keys as secrets, check, recording handed over | done, draft, provisional |
| `rpc`: an item too large for any message no longer blocks what is behind it (`Problem::next`), error texts are cut at 64 KiB, `run_until_signal` waits for `run_to_end` work, `prepare_data_dir` answers the canonical path, `--instance` must be a domain | done, version 0.32.0 |
| `files` gains `BlobBackend.inspect @16` | done |
| `texts` draft 0.2 (0.50.0): history, trash, spaces and ordered folders, templates, redirects, search with snippets, typed conflict problem; `TextEntry.template/redirectTo/updatedBy`; methods `@16` to `@35`; append only (section "texts 0.2") | done, draft |
| Generated Rust with `capnp-compiler` v0.1.0 + `capnpc` 0.27 | done; byte-identical to `capnpc-embedded` 0.2 output (tested) |
| Rust helpers `Unsupported`, `Capabilities` with wire round trips | done |
| `rpc` (feature): socket with lock file, connection, problems, envelope, pages, feed | done, draft |
| `rpc::client` (feature): `Worker<R: Remote>` (thread, current-thread runtime + `LocalSet`, command queue, `Timeouts` 5 s connect / 30 s call, drop cuts the connection first (commands under way get `Disconnected`, a write may be left half done), then joins; a command that waits on nothing from the connection keeps the drop waiting up to the call deadline; on the connection thread itself nothing is joined), `call_limited(server, limit, future)` gives any call on any connection (also the bare `Connection` after `connect_matching`) a deadline and a typed `Fault::TimedOut`; the timed-out call is dropped, the connection stays usable (v0.0.18); reply guard (`Fault::Interrupted`), `Fault` naming the deadline, `problems` (`Failure::of`, `local_type`, `first_number`, `object_id_in`, `without_store_prefix`, `unsupported_of`); taken from the people and tasks clients, which map to their own `Error` and problem type URIs themselves | done, draft; tests in `tests/rpc_client*.rs` against a real server |
| Tests | 2 oracle, 3 provider, 40 `rpc` (+21 problem extensions and text format), 19 `rpc::client` (+2 unit), 1 doc test |
| `types.capnp` (new, draft 0.1, 2026-11-10; ADR 0066): `BasicType` (email, phone, name, address, place, date, date-time, language, link, image, key, choice, keywords, reference, text), the type description (`TypeDescription`, `FieldDescription`, `CommandDescription`, `CommandKind`) and `TypeCatalog.describe`; Rust side in the workspace crate `fs-types` (see `crates/fs-types/DESIGN.md`: where it lives, pattern map, free crate `calcard` for JSContact/vCard, `Unsupported` list) | new, draft, 0 of 2 consumers |
| `rpc::run_until_signal` registers SIGTERM and SIGINT before it binds the socket and opens the store (a stop during the start was a hard kill; found by thread P under load; test `rpc_signal_start.rs`, which dies by SIGTERM without the fix). A signal during the start is remembered and ends the serving as soon as it begins | done |
| Interfaces at 1.0 | none; the draft -> 1.0 gate needs two independent consumers per interface |

## texts 0.2 (contract in this repo; core and server in fs-wiki, 1 of 2 consumers)

All of it is append only: `TextEntry` gains `template @4`, `redirectTo @5`, `updatedBy @6`; `Texts` gains methods `@16` to `@35`; the 16 old methods keep their ordinals (test `texts_contract.rs` pins this). Each feature has a capability id in `supportReport` (the standard way, see Decisions). A provider that lacks a feature answers `Unsupported` for that id; an old server answers a new method as unimplemented (`Problem` 501), which a client treats the same. Zero `Uuid` means "none". `Expect` (`any` | `revision`) is the compare-and-set argument of the new writes; `editIf @15` stays.

**Typed problems.** Types are consts in `texts.capnp` (`notFoundProblemType` ... `notEmptyProblemType`), the same style as `files`. The conflict of `editIf`, `editWith`, `restoreRevision`, `setTemplate`, `setRedirect` is `conflictProblemType` (409) with extensions `id` (the text), `kind` = `text` (the 0.1 methods, `editIf` included, may keep `page`), `issue` = `stale-revision`, `revision` = the current revision when known, `expected` = what the caller sent; `detail` still starts `conflict:` for 0.49 readers. Everything else a client needs is read from `type` and the extensions, never from `detail`.

| Method | Capability | Semantics and limits |
|---|---|---|
| `editWith(id, expect, document, summary)` | `text.editSummary` | `edit`/`editIf` plus a revision summary (at most 200 bytes, else `invalid`, issue `summary-too-long`). |
| `history(id, cursor)` | `text.history` | Revisions of a text, newest first, `RevisionInfo` (number, revision, author, at, size, summary, path, restoredFrom). At most 100 per page, opaque cursor. Works for a trashed text. `number` counts from 1 per text, never reused; it (not `revision`) orders. A provider keeps per text at least the last 50 revisions or 90 days of them, whichever is more (the current one always). Every write adds one: create, edit*, move, import, restoreRevision, restoreFromTrash, setTemplate, setRedirect, and a link rewritten by a move in the other texts; a no-op setTemplate/setRedirect adds none. `number` 1 is the first known state: a text written before history existed starts at its first write after the upgrade. |
| `getRevision(id, number)` | `text.history` | The document of one revision (`found = false` if it never existed or was pruned). The answer must fit one message (`too-large` otherwise, as `get`). |
| `restoreRevision(id, number, expect)` | `text.history` | Writes the old document as a new revision (`restoredFrom = number`, summary `restored revision N`). Path, template and redirect flags stay as they are now. History is never rewritten or truncated by it. |
| `delete(id)` (existing) | `text.delete` | Keeps its 0.1 meaning exactly; this contract does not change it. |
| `trash(id)` | `text.trash` | Moves the text and its history to the trash: `get`, `list`, `find`, `search`, `links` no longer show it, its path is free again, a feed event `Deleted` is told. A text already in the trash (or unknown) answers `not-found`, kind `text`. Returns the `TrashItem`. |
| `listTrash(cursor)` | `text.trash` | `TrashItem` (id, path, name, deletedAt, deletedBy, size), most recently deleted first, paged. |
| `restoreFromTrash(id, path)` | `text.trash` | Empty path = the old path. If the path is taken: `path-taken` (409), nothing changes; the caller may send another path. Not in trash: `not-in-trash`. |
| `purge(id)` | `text.trash` | Removes one trashed text for good; scope below. Refuses a text not in the trash (`not-in-trash`). Nothing purges implicitly through this API (a provider may expire the trash by its own policy and must say so in its report). |
| `purgeRevision(id, number)` | `text.history` (risk: destructive) | Removes ONE old revision for good, for a leaked secret. Not the current revision (`invalid`, issue `current-revision`; edit the text first, then purge the old one). The revision `number` stays used: the gap is shown as a gap, `getRevision` answers `found = false`, `restoreRevision` of it answers `not-found` kind `revision`, new numbers continue after the highest ever given. Text of the secret that is still in later revisions is not found or cleaned by this call. |
| `createFolder(parent, name, path)` | `text.space` | Zero parent = a new space. Folder name at most 255 bytes, no `/`. An explicit `path` must lie strictly below the parent's path (`parent.path + "/" + ...`; for a space any valid path) and obey the path rules: no `.` or `..` segment, no empty segment, no leading or trailing `/`, no U+10FFFF; else `bad-path` (issue = the reason alone: `outside-parent`, `dot-segment`, ..., ext `path`). Empty path = parent path + `/` + slug of the name; a name with an empty slug (for example only non-Latin letters) answers `bad-path`, issue `empty-slug`, unless a path is given. **Adoption:** creating a folder over a prefix that already holds texts (or exists as a path only) adopts it: the texts stay, only the node is new. A prefix already owned by a folder node: `path-taken`. |
| `renameFolder(id, name)` | `text.space` | Display name only. A path change is `moveFolder` (links follow, as before). |
| `deleteFolder(id)` | `text.space` | Only an empty folder (`not-empty`, 409): texts in the trash below it still count, so they block it until purged or restored. Never deletes texts. |
| `getFolder(id)` | `text.space` | `Node` of a space or folder. |
| `children(parent, cursor)` | `text.space` | Ordered children (folders, pages) as `Node`s with `position`; zero parent = spaces and pages outside every folder. At most 200 per page. Children never placed come after placed ones, in creation order. |
| `reorder(id, placement, anchor)` | `text.space` | `first`, `last`, `before`, `after` (anchor must be a sibling, else `invalid`, issue `anchor`) within the current parent. Moving to another parent is `move`/`moveFolder`; the page's order slot then is `last` there. The order is state of the provider, not part of the document. |
| `setTemplate(id, template, expect)` | `text.template` | Marks or unmarks. A template is an ordinary text (listed, edited, versioned) that `search` can include or exclude. |
| `createFromTemplate(template, path)` | `text.template` | New text with a copy of the template's document, new id, not a template, own history starting with summary `from template <path>`. No placeholder substitution. Path taken: `path-taken`; source not a template: `invalid`, issue `not-a-template`. |
| `setRedirect(id, target, expect)` | `text.redirect` | Target is a path (empty clears). The target must be a valid internal path under the same path rules (no scheme such as `https:`, no `..`); else `bad-path` (issue `external-target` or the rule, ext `path`). The target need not exist. Target = own path: `invalid`, issue `self-redirect`. `get`/`getByPath` never follow. |
| `resolve(path)` | `text.redirect` | Follows redirects, at most 8 (`maxRedirectHops`). `Resolution.outcome`: `resolved` (with the final text), `notFound`, `loop` (a path came twice; it wins whenever a cycle is detected), `tooManyHops` (more than 8 redirects, no cycle seen), `unknown` (ordinal 0: an unset or newer value, never success). `hops` lists the paths visited, start first; for `resolved` the final path is not in it. These are results, not problems. |
| `search(query, cursor)` | `text.search` | Term (all words, case-insensitive, at most 256 bytes and 8 words, else `invalid`, issue `term-too-long` or `too-many-words`); `prefix = true` matches the last word as a prefix, at least 2 bytes (`prefix-too-short`). Filters (AND): `space`, `pathPrefix`, `template`/`redirect` (`any`/`only`/`exclude`), `author` (last writer), `updatedFrom`/`updatedTo` (half open, 0 = none). Hits carry at most 3 snippets of at most 240 bytes with at most 20 highlights each (UTF-8 byte ranges, on character boundaries, ordered, not overlapping). At most 50 hits per page. Order: (relevance desc, updated desc, id asc); with an empty term relevance is equal. The cursor continues after that tuple of the last hit; a text that changes between two pages may be skipped or shown twice (relevance moves). `find` stays as the simple search. |

### Purge scope (provisional, owner may change)

`purge` removes from the provider: the text object, its log entries, all its revisions, its entries in the search and link indexes, and its files (provider: `purge_object` plus `clean_files`). It does **not** reach copies that already left the provider: what was synced to the hub or told on the feed stays there (the hub needs a purge of its own, `sync_then_purge` in fs-hub), and an unsynced delete may never reach the hub. A client must not promise a person "gone everywhere".

### Purge through the hub (decided)

The wiki goes through `sync_then_purge` of fs-hub, not `purge_object` directly. `purgeRevision` purges the whole object log of that text and therefore ends undo of that text; it needs the hub way too.

### Where revisions live (provisional)

`number` and the revisions are stored with the text's history, **not** as a counter inside the text record: a store undo or a log compaction/clear-history would reset such a counter and reuse numbers. The provider derives the next number from the highest ever given (kept apart from the revisions it may prune).

### Trash edge cases (provisional)

* A trashed text counts as a child for `deleteFolder` (blocks it) until purged or restored; so a restore never lands in a deleted folder through this API. If the folder node is gone anyway, `restoreFromTrash` keeps the path and the page has the nearest existing folder as parent (or none); it is not moved.
* `trash` of a trashed or unknown text -> `not-found`; `purge` of a live text -> `not-in-trash`; `purge` of an unknown or already purged text -> `not-found`.
* Path errors: only the 0.2 methods answer `bad-path`; the 0.1 methods answer `invalid`. The `issue` of `bad-path` is the reason alone.
* `moveFolder` moves the folder nodes at and below the prefix with the texts, and keeps their order slots.
* Links rewritten by a `move` are writes in the other texts and add a revision there (summary `links updated`); a provider that cannot do that says so in its report.
* Structure changes (folders, order, flags, trash and restore) have **no feed events yet** (`subscribe` tells text changes only); open point.
* Redirects do **not** follow page moves (a redirect stays a path); open point, decision.

### Open points of texts 0.2 (choices I was unsure about; for the strategy session)

0. All decisions marked provisional above and in the list are the coordinator's and may be changed by the owner. `revision` in the conflict is "when known": fs-tasks and fs-people omit it on purpose; align all three later. `kind` values of `not-found`: `text`, `folder`, `revision`.
1. **`kind` in the conflict problem.** The order said `kind=conflict`; `rpc::problem_keys::KIND` is defined as the noun of the object (`task`, `page`), and the problem `type` already says conflict. I set `kind` = `text`, `issue` = `stale-revision` and added `revision`/`expected`. Say if `kind` should be `conflict`.
2. **`editWith`/`Expect`** is a new write method instead of a `summary` on `editIf` (parameters added to an existing method would change meaning for old callers). The other new writes take `Expect` rather than a bare revision. Alternative: summary-less only.
3. **`delete` keeps its meaning;** the separate `trash` method (`@35`) is the recoverable one. `delete` keeps every revision of the text and no method removes them; a caller who wants removal for good must `trash` then `purge`. Decided, provisional.
4. **No `emptyTrash`, no retention setting** in the contract; only per-text `purge`. Provider-side expiry is allowed but undefined.
5. **History retention:** pruning is allowed (gaps in `number`); the obligation is at least 50 revisions or 90 days per text, whichever keeps more. Provisional.
6. **Folders are nodes at path prefixes** (a folder owns a prefix, a page's parent is its longest-prefix folder), not a second tree independent of paths. It keeps `move`/links as they are but means a page cannot sit in a folder whose prefix is not its path. Order slots are per parent; `position` is an index, not a stable key.
7. **Order on `move` across parents** (slot `last`) and `children` cursor stability under concurrent reorders are not specified beyond "opaque cursor"; the cursor limits above (`Limits of rpc`) apply.
8. **Template creation** copies without placeholders; no template parameters (title, date).
9. **Search `author`** compares the last writer only, not any past author; **no relevance score** on the wire; `field` is free text (`name`, `path`, `body`) rather than an enum so a provider can add `tag` later.
10. **Redirect to a path** (not to an id): it survives a recreate and follows `move` only if the provider rewrites it as it rewrites links; the contract does not yet require that.
11. **Search over trashed texts** is not offered (no `trashed` filter); `listTrash` has no term.
12. Existing data has paths only; `createFolder` over an existing prefix adopts it (decided above).
13. `Timestamp` millisecond resolution and the `Time` limit above still apply to `at`, `updatedFrom`, `updatedTo`.

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

* Owner of `common.capnp` (`fs-objects` is proposed in ADR 0050).
* More topics from the catalogue (people, groups, events, files ...) as their owners write drafts.

## fs-mcp

The MCP tool generator (stage 1, lists only) lives in `crates/fs-mcp`; see its `DESIGN.md`.

## Contact values (feature `values`, 0.51.0)

`values::{EmailAddress, PhoneNumber, ContactValue, ValueProblem}`: value objects shared by all programs (e-mail by the practical HTML rule with IDNA-mapped domains and the 254-character limit; phone E.164). Equality and hash follow the canonical form. A failure is a code, the sentence is the program's. Not a contract: no schema, no draft status.


## Windows transport (2026-11-10, sprint decision C: a real port, no cross-build)
`rpc/transport/` has one trait, `Endpoint` (Strategy), and two implementations; `rpc::serve`, `connect`, `bind_socket`, `default_socket`,
`ServerOptions` and the data directory go through it and name neither. **Unix** (`transport/unix.rs`): a Unix socket, a lock file for the
single instance, `SO_PEERCRED`, SIGTERM/SIGINT, ancestors checked. **Windows** (`transport/windows.rs`): a named pipe
`\\.\pipe\freesynergy-<user>-<program>` (a `--socket PATH` that is not a pipe name becomes one), `first_pipe_instance` is the single-instance
rule (a second start gets `AddrInUse`), `reject_remote_clients`, Ctrl-C/Break/close/shutdown, data in `%LOCALAPPDATA%\FreeSynergy\<program>`
made private through the security API. New: `stop_self()` ends `run_until_signal` in order from inside (a lease that ran out; Windows has no signal to
send to oneself) and `companion_path(socket, "lease")` (files that belong to a socket; a pipe has no folder).
**Known differences on Windows:** a named pipe lives as long as one end
is open, so after `serve` returned the name is free only when the served connections are gone; the checks of group-writable ancestors are
Unix concepts and are not made. The Windows gate is `.github/workflows/windows.yml` (own steps: it lints and tests the crate `fs-interfaces`
only, because the pedantic clippy of `fs-types` already fails on `main`).

**Hardening and stop (2026-11-10, after the review):** the pipe is `\\.\pipe\freesynergy-<sid>-<program>` (SID of the process token; names are encoded so that two texts never meet, case does not matter, long ones carry a hash). Every instance has a protected descriptor with one entry (this user), and both ends check the process on the other end (`GetNamedPipeClientProcessId` on the server, `GetNamedPipeServerProcessId` on the client; test on both). The accept loop skips a caller that hung up before it (`ERROR_NO_DATA`) and replaces the spent instance, makes the next instance before it hands the served one over (a failure there is a pause), and a client asks again briefly on `NotFound`. The data folder gets the same descriptor through the API (no `icacls`), an existing folder must be owned by this user, and the path comes back without `\\?\`; files of a pipe live in `%LOCALAPPDATA%\FreeSynergy\run` or else a private folder in the temporary one. **Stop from outside:** the interface `Control.stop` (`common.capnp`) is answered by `run_until_signal` next to the program's own capability (a wrapper that passes every other call on unchanged); `rpc::stop_server(socket)` asks. Unix keeps SIGTERM. `stop_self` belongs to one run (forgotten when the run ends). `--facet NAME` is read by one parser, `rpc::parse_facet` / `facet_from_args`, for `ServerOptions` and for the apps.

**Version handshake (2026-11-10, branch `rpc-version-handshake`):** after an update an old server (`fs-files serve` 0.0.2) can still run, and the new window got "Method not implemented" and stayed black. `Control.version @1` (append only: program, version, `PROTOCOL` level) is answered by `run_until_signal_as(socket, ServerIdentity::new(name, version), open)` (`run_until_signal` still works and says nothing about the version). `rpc::connect_matching(socket, &Expected::new(env!("CARGO_PKG_VERSION")), &starter)` replaces `connect` for a program's client: a matching server is used without interruption; none running: the program's `ServerStarter` (a closure or `CommandStarter::new(current_exe, ["serve"])`) starts one; a server that differs or does not know `Control.version`: `Control.stop`, and for a server that does not know `Control` at all (`stop_server` answers `io::ErrorKind::Unsupported`, no longer `Other`) SIGTERM to the process found by `SO_PEERCRED` (Unix only; Windows has no such signal, so there it is `CannotRestart` with "end it by hand"), then wait until the socket is free (10 s), start, connect again, compare again. Failure is a typed `HandshakeError`: `CannotRestart { running, expected, reason }`, `ServerMismatch { running, expected }` (the new server is still wrong), `Io`. **Where it is connected:** each program's client (`fs-<program>` client, the `Remote` of `rpc::client`) calls `connect_matching` instead of `connect` and hands the error to the window of `fs-components`, which must show `error.to_string()` as a persistent message with "try again" (ADR 0065 §4, §5): never a black window. Not built here: the window, the `ServerStarter` of each program.

**Handshake review notes (2026-11-10):** a server of plain `serve()` (no `Control`) is treated as old and replaced by SIGTERM; the signal is never sent to this process or to PID 1 (`Endpoint::terminate` refuses). Two clients at once: before `Control.stop` the version is asked again, a server that fits by then is used, "gone already" counts as ended, and the wait for the free socket also accepts a fitting new server (test with two clients). `CommandStarter` starts the server in its own session on Unix (`setsid`) and collects the child's exit (no zombie); on Windows `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP`. **Release note:** `stop_server` answers `io::ErrorKind::Unsupported` (was `Other`) for a server that does not know `Control`; callers that matched on `Other` must match `Unsupported`. No CHANGELOG file exists in this repository. **Follow-up:** (3) the PID found through `SO_PEERCRED` can be reused between lookup and signal; a pidfd (`pidfd_open`) taken at the lookup would close that gap. (4) as the coordinator's review item 4, not done here.
