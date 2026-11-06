# fs-wiki contract — DRAFT 0.1 (2026-11-06)
#
# STATUS: DRAFT. Owner program: fs-wiki. Lives in fs-interfaces (ADR 0045, 0050).
# Consumers so far: none (0 of 2). Provider: fs-wiki-core over the text library fs-text.
# Every text is a wiki entry (ADR 0031): text + layers, the database is the truth, files are exports
# (ADR 0042). A text travels as the JSON form of its document, with its AS2 envelope; Markdown goes
# in and out through import and export. Evolution rules: only append.

@0xba5065cd4ae83ff8;

using C = import "common.capnp";
using Uuid = C.Uuid;
using Timestamp = C.Timestamp;
using Cursor = C.Cursor;
using Envelope = C.Envelope;
using SupportReport = C.SupportReport;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;

struct TextEntry {
  envelope @0 :Envelope;           # AS2 `Article` or `Note`
  path     @1 :Text;               # readable path, e.g. "manual/start"; unique
  document @2 :Text;               # the document as JSON (authoritative content)
}

struct TextPage { items @0 :List(TextEntry); next @1 :Cursor; }

struct LinkRow {
  source @0 :Uuid;
  target @1 :Text;                 # readable path of the target
  at     @2 :Text;                 # where in the source
}

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
}
