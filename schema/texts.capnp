# fs-wiki contract — DRAFT 0.2 (2026-11-08)
#
# STATUS: DRAFT. Owner program: fs-wiki. Lives in fs-interfaces (ADR 0045, 0050).
# Consumers so far: none (0 of 2). Provider: fs-wiki-core over the text library fs-text.
# Every text is a wiki entry (ADR 0031): text + layers, the database is the truth, files are exports
# (ADR 0042). A text travels as the JSON form of its document, with its AS2 envelope; Markdown goes
# in and out through import and export. Evolution rules: only append.
#
# DRAFT 0.2 adds (all append only; a provider that lacks one answers `Unsupported` for its
# capability id in `supportReport`): history (`text.history`), trash (`text.trash`), spaces and
# ordered folders (`text.space`), templates (`text.template`), redirects (`text.redirect`), search
# with snippets and filters (`text.search`), and typed problems (below). Semantics and limits of each
# method: fs-interfaces DESIGN.md, section "texts 0.2". Zero `Uuid` (hi = lo = 0) means "none".
#
# PROBLEMS. Besides `unsupportedProblemType` (common.capnp) the methods answer these RFC 9457
# problem types (`Problem.type`, with `status`, `detail`, and the extension members named):
#   notFoundProblemType    404  id, path, revision number or folder does not exist (ext `id`, `kind`)
#   conflictProblemType    409  `editIf`, `editWith`, `restoreRevision`, `setTemplate`, `setRedirect`:
#                               the text no longer has the expected `revision`. Nothing was written.
#                               ext: `id` (the text), `kind` = "text", `issue` = "stale-revision",
#                               `revision` = the CURRENT revision as decimal u64 (key
#                               `conflictRevisionKey`), `expected` = the revision the caller sent
#                               (key `conflictExpectedKey`). `detail` still starts `conflict:` so a
#                               0.49 reader that looks at the text keeps working.
#   pathTakenProblemType   409  the path (or the original path on restore) is used by another text
#   moveRefusedProblemType 409  a move is refused (target not empty, another move unfinished)
#   invalidProblemType     422  the request is not acceptable; one `issue` per reason (for example
#                               `term-too-long`, `summary-too-long`, `self-redirect`, `bad-path`)
#   notInTrashProblemType  409  `purge` or `restoreFromTrash` on a text that is not in the trash
#   notEmptyProblemType    409  `deleteFolder` on a folder that still has children (ext `id`)
# A client reads `type` and the extension keys, never the sentence in `detail`.

@0xba5065cd4ae83ff8;

using C = import "common.capnp";
using Uuid = C.Uuid;
using Timestamp = C.Timestamp;
using Cursor = C.Cursor;
using Envelope = C.Envelope;
using SupportReport = C.SupportReport;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;

const notFoundProblemType    :Text = "https://freesynergy.net/problems/texts/not-found";
const conflictProblemType    :Text = "https://freesynergy.net/problems/texts/conflict";
const pathTakenProblemType   :Text = "https://freesynergy.net/problems/texts/path-taken";
const moveRefusedProblemType :Text = "https://freesynergy.net/problems/texts/move-refused";
const invalidProblemType     :Text = "https://freesynergy.net/problems/texts/invalid";
const notInTrashProblemType  :Text = "https://freesynergy.net/problems/texts/not-in-trash";
const notEmptyProblemType    :Text = "https://freesynergy.net/problems/texts/not-empty";
# extension keys of conflictProblemType (the standard keys `id`, `kind`, `issue` are in rpc::problem_keys)
const conflictRevisionKey    :Text = "revision";
const conflictExpectedKey    :Text = "expected";
const staleRevisionIssue     :Text = "stale-revision";
# most redirects `resolve` follows before it answers `tooManyHops`
const maxRedirectHops        :UInt8 = 8;
# bounds a provider enforces (a larger request is `invalidProblemType`); see DESIGN.md "texts 0.2"
const maxSummaryBytes        :UInt16 = 200;
const maxSearchTermBytes     :UInt16 = 256;
const maxSnippetBytes        :UInt16 = 240;
const maxSnippetsPerHit      :UInt8 = 3;
const maxHighlightsPerSnippet :UInt8 = 20;
const maxHistoryPage         :UInt16 = 100;
const maxChildrenPage        :UInt16 = 200;
const maxSearchPage          :UInt16 = 50;
const maxFolderNameBytes     :UInt16 = 255;

struct TextEntry {
  envelope @0 :Envelope;           # AS2 `Article` or `Note`
  path     @1 :Text;               # readable path, e.g. "manual/start"; unique
  document @2 :Text;               # the document as JSON (authoritative content)
  revision @3 :UInt64;             # changes at every write of this text (for editIf); compare for equality only: it is no counter, so it says nothing about order or number of writes
  template @4 :Bool;               # 0.2: marked as a template (text.template); false from a provider without it
  redirectTo @5 :Text;             # 0.2: path this text forwards to; empty = not a redirect (text.redirect)
  updatedBy @6 :Text;              # 0.2: actor (IRI or DID) of the last write; empty = not known
}

struct TextPage { items @0 :List(TextEntry); next @1 :Cursor; }

struct LinkRow {
  source @0 :Uuid;
  target @1 :Text;                 # readable path of the target
  at     @2 :Text;                 # where in the source
  title  @3 :Text;                 # the link text as written; empty = none
  internal @4 :Bool;               # true: the target is a text of this provider; false: an outside address
}

# ---- 0.2: what a write expects --------------------------------------------------------------
# `any` (ordinal 0, so the default) = write whatever the text is now. `revision` = only if the text
# still has that revision, else `conflictProblemType` and nothing is written.
struct Expect { union { any @0 :Void; revision @1 :UInt64; } }

# ---- 0.2: history (text.history) -------------------------------------------------------------
# Every write of a text (create, edit, editIf, editWith, move, import, restoreRevision, setTemplate,
# setRedirect) adds one revision. `number` counts them per text from 1, never reused, never
# rewritten; it is the only thing here that orders revisions (`TextEntry.revision` does not).
struct RevisionInfo {
  text     @0 :Uuid;
  number   @1 :UInt64;             # 1 = the creation; gaps are possible where a provider pruned old ones
  revision @2 :UInt64;             # the `TextEntry.revision` this write produced
  author   @3 :Text;               # actor (IRI or DID); empty = not known
  at       @4 :Timestamp;
  size     @5 :UInt64;             # bytes of the document JSON of this revision
  summary  @6 :Text;               # what the writer said (editWith), else the provider's own short note ("created", "moved", "restored revision 4"); at most maxSummaryBytes
  path     @7 :Text;               # the text's path at that time
  restoredFrom @8 :UInt64;         # the `number` this revision was restored from; 0 = not a restore
}
struct RevisionPage { items @0 :List(RevisionInfo); next @1 :Cursor; }   # newest first
struct RevisionContent { info @0 :RevisionInfo; document @1 :Text; }

# ---- 0.2: trash (text.trash) -----------------------------------------------------------------
struct TrashItem {
  id        @0 :Uuid;
  path      @1 :Text;              # the path it had when deleted
  name      @2 :Text;              # derived name (Envelope.name)
  deletedAt @3 :Timestamp;
  deletedBy @4 :Text;
  size      @5 :UInt64;            # bytes of the document JSON
}
struct TrashPage { items @0 :List(TrashItem); next @1 :Cursor; }          # most recently deleted first

# ---- 0.2: spaces, folders and the order of siblings (text.space) -----------------------------
# A node is a space (top level), a folder, or a page (a text). Folders and spaces have an id and a
# display `name`; their `path` is the prefix they own (space "manual" owns "manual/..."). A page's
# parent is the folder whose path is the longest prefix of the page's path, so moving a page between
# folders is the existing `move`/`moveFolder`; only the ORDER among siblings is new state.
enum NodeKind { page @0; folder @1; space @2; }
struct Node {
  id       @0 :Uuid;               # the text id for a page
  kind     @1 :NodeKind;
  name     @2 :Text;               # display name (folder: as given; page: Envelope.name)
  path     @3 :Text;               # folder/space: its prefix without trailing "/"; page: its path
  parent   @4 :Uuid;               # zero = top level (a space, or a page outside every folder)
  position @5 :UInt32;             # 0-based index among the siblings, as of this answer; not stable across writes
  template @6 :Bool;               # pages only
  redirect @7 :Bool;               # pages only
}
struct NodePage { items @0 :List(Node); next @1 :Cursor; }
enum Placement { first @0; last @1; before @2; after @3; }

# ---- 0.2: redirects (text.redirect) ----------------------------------------------------------
enum Outcome { resolved @0; notFound @1; loop @2; tooManyHops @3; }
struct Resolution {
  outcome @0 :Outcome;             # resolved: `text` is the end of the chain (possibly the start itself, hops empty)
                                   # notFound: the start or a target does not exist; `hops` ends at the missing path
                                   # loop: a path came up twice; tooManyHops: more than maxRedirectHops redirects
  hops    @1 :List(Text);          # the paths followed, start first, the final path excluded
  text    @2 :TextEntry;           # set only when outcome = resolved
}

# ---- 0.2: search with snippets and filters (text.search) -------------------------------------
enum Tri { any @0; only @1; exclude @2; }       # any = no restriction (the default)
struct SearchFilter {
  space         @0 :Uuid;          # only texts below this space or folder; zero = everywhere
  pathPrefix    @1 :Text;          # only texts whose path starts with this (exact, case sensitive); empty = no restriction
  template      @2 :Tri;           # only / exclude templates
  redirect      @3 :Tri;           # only / exclude redirects (the default `any` returns them too)
  author        @4 :Text;          # last writer (TextEntry.updatedBy), compared exactly; empty = no restriction
  updatedFrom   @5 :Timestamp;     # updated at or after; unixMillis 0 = no restriction
  updatedTo     @6 :Timestamp;     # updated before (half open); unixMillis 0 = no restriction
}
struct SearchQuery {
  term   @0 :Text;                 # words, case-insensitive, all must match; at most maxSearchTermBytes; empty = filters only
  prefix @1 :Bool;                 # true: the LAST word matches as a prefix (type-ahead); the others whole
  filter @2 :SearchFilter;
}
# A range of a snippet in UTF-8 bytes, half open, always on character boundaries.
struct Highlight { start @0 :UInt32; end @1 :UInt32; }
struct Snippet {
  field      @0 :Text;             # "name" | "path" | "body"
  text       @1 :Text;             # at most maxSnippetBytes, cut on a character boundary, never inside a highlight
  highlights @2 :List(Highlight);  # ordered, not overlapping, at most maxHighlightsPerSnippet
  cutBefore  @3 :Bool;             # text continues before this snippet (show an ellipsis)
  cutAfter   @4 :Bool;
}
struct SearchHit {
  id        @0 :Uuid;
  path      @1 :Text;
  name      @2 :Text;
  updated   @3 :Timestamp;
  updatedBy @4 :Text;
  template  @5 :Bool;
  redirect  @6 :Bool;
  snippets  @7 :List(Snippet);     # at most maxSnippetsPerHit; empty for a filters-only search
}
struct SearchPage { items @0 :List(SearchHit); next @1 :Cursor; }

interface Texts {
  supportReport @0 () -> (report :SupportReport);
  create    @1 (path :Text, markdown :Text) -> (text :TextEntry);                    # text.create
  edit      @2 (id :Uuid, document :Text) -> (text :TextEntry);                      # text.edit
  move      @3 (id :Uuid, path :Text) -> (text :TextEntry);                          # text.move: links follow
  delete    @4 (id :Uuid) -> ();                                                 # text.delete
  get       @5 (id :Uuid) -> (found :Bool, text :TextEntry);                         # text.read
  getByPath @6 (path :Text) -> (found :Bool, text :TextEntry);                       # text.read
  list      @7 (prefix :Text, cursor :Cursor) -> (page :TextPage);               # text.read
  find      @8 (term :Text, cursor :Cursor) -> (page :TextPage);                 # text.find
  links     @9 (id :Uuid) -> (items :List(LinkRow));                             # link.read
  backlinks @10 (path :Text) -> (items :List(LinkRow));                          # link.read
  importText @11 (path :Text, mediaType :Text, text :Text) -> (entry :TextEntry);    # text.import: an outside change is a normal edit
  exportText @12 (id :Uuid, mediaType :Text) -> (text :Text);                    # text.export
  subscribe  @13 (listener :ChangeListener) -> (subscription :Subscription);     # text.subscribe
  moveFolder @14 (prefix :Text, path :Text) -> (moved :List(TextEntry));          # text.move: the text at `prefix` (if any) and every `prefix/...` move along; links follow; a clash below refuses all, nothing written
  editIf     @15 (id :Uuid, revision :UInt64, document :Text) -> (text :TextEntry);  # text.edit, only if the text still has `revision`; otherwise it fails with `conflictProblemType` (typed, see PROBLEMS above; detail still starts `conflict:`) and writes nothing (compare-and-set; atomic inside one provider process only)
  # ---- 0.2, append only ---------------------------------------------------------------------
  editWith   @16 (id :Uuid, expect :Expect, document :Text, summary :Text) -> (text :TextEntry);   # text.edit: edit/editIf with a revision summary (may be empty; at most maxSummaryBytes)
  # history (text.history). Also readable for a text in the trash.
  history    @17 (id :Uuid, cursor :Cursor) -> (page :RevisionPage);                          # text.history: newest first, at most maxHistoryPage per page
  getRevision @18 (id :Uuid, number :UInt64) -> (found :Bool, revision :RevisionContent);     # text.history: found = false if never existed or pruned
  restoreRevision @19 (id :Uuid, number :UInt64, expect :Expect) -> (text :TextEntry);        # text.history: writes the old document as a NEW revision (restoredFrom = number); path, template and redirect stay as now; history is never rewritten
  # trash (text.trash). With this capability `delete @4` moves the text to the trash.
  listTrash  @20 (cursor :Cursor) -> (page :TrashPage);                                       # text.trash
  restoreFromTrash @21 (id :Uuid, path :Text) -> (text :TextEntry);                           # text.trash: empty path = the old one; taken = pathTakenProblemType, nothing changes
  purge      @22 (id :Uuid) -> ();                                                            # text.trash: forever, only for a text in the trash (else notInTrashProblemType); the one way to destroy a text
  # spaces and ordered folders (text.space)
  createFolder @23 (parent :Uuid, name :Text, path :Text) -> (node :Node);                    # text.space: zero parent = a new space; empty path = parent path + "/" + a slug of name
  renameFolder @24 (id :Uuid, name :Text) -> (node :Node);                                    # text.space: display name only; a path change is moveFolder
  deleteFolder @25 (id :Uuid) -> ();                                                          # text.space: only an empty folder (notEmptyProblemType); never deletes texts
  getFolder  @26 (id :Uuid) -> (found :Bool, node :Node);                                     # text.space
  children   @27 (parent :Uuid, cursor :Cursor) -> (page :NodePage);                          # text.space: in order; zero parent = the spaces and pages outside any folder; at most maxChildrenPage
  reorder    @28 (id :Uuid, placement :Placement, anchor :Uuid) -> (node :Node);              # text.space: within its current parent; anchor needed for before/after and must be a sibling (else invalidProblemType, issue `anchor`)
  # templates (text.template) and redirects (text.redirect)
  setTemplate @29 (id :Uuid, template :Bool, expect :Expect) -> (text :TextEntry);            # text.template
  createFromTemplate @30 (template :Uuid, path :Text) -> (text :TextEntry);                   # text.template: copy of the template's document with a new id; not a template; own history
  setRedirect @31 (id :Uuid, target :Text, expect :Expect) -> (text :TextEntry);              # text.redirect: empty target clears; the target need not exist; itself = invalidProblemType, issue `self-redirect`
  resolve    @32 (path :Text) -> (resolution :Resolution);                                    # text.redirect: follows redirects, at most maxRedirectHops; `get`/`getByPath` never follow
  # search (text.search)
  search     @33 (query :SearchQuery, cursor :Cursor) -> (page :SearchPage);                  # text.search: relevance first (filters only: updated, newest first); at most maxSearchPage hits
}
