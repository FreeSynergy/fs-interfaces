# fs-whiteboard contract — DRAFT 0.1 (2026-11-06)
#
# STATUS: DRAFT, PROVISIONAL. Owner program: fs-whiteboard. Lives in fs-interfaces (ADR 0045).
# Consumers so far: none (0 of 2). No open standard describes a board as a queryable model, so this is
# our own specification and a candidate for submission (ADR 0014;
# docs/en/programs/apps/whiteboard/design.md).
#
# Every element is an object with an id, on an infinite canvas: coordinates are Float64 in board units;
# NaN and infinities are refused. Merging is per property: each property has a logical clock, so one
# person's colour change does not undo another's move. The clocks and the merge belong to the stored
# form, and travel with it, not with this interface; `sync` between devices is not in this contract
# yet (the hub delivers only other programs' activities). The merge units are the members of `Element`,
# each one of `Style`, and `Page` by itself; they are listed in fs-whiteboard/DESIGN.md.
#
# The points of a finished stroke never change, so they cannot conflict; they are relative to the
# frame, which can move and scale. A deleted element stays as a marker with its properties, so a
# delete is itself a property that merges, and undo can bring it back. A deleted group or frame hides
# its members, also ones added concurrently. A connector whose end is deleted keeps that end at the
# element's last frame. A removed page hides its elements; if concurrent removals leave none, the page
# with the highest id stays. A cycle formed by two concurrent changes of `parent` is broken at reading,
# the same way everywhere: the element with the lowest id on the cycle counts as having no parent.
#
# Passing data such as cursors is not here and is never stored: it belongs to the shared Presence
# interface, which does not exist yet. Images are IRIs of files.capnp; they are only referred to.
#
# Evolution rules (Cap'n Proto): never renumber or remove fields/methods; only append.

@0xc72e9b14a8f3d605;

using C = import "common.capnp";
using Uuid = C.Uuid;
using Cursor = C.Cursor;
using Envelope = C.Envelope;
using CapabilityList = C.CapabilityList;
using SupportReport = C.SupportReport;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;

struct Point { x @0 :Float64; y @1 :Float64; }

# one point of a stroke, relative to the frame; pressure 0..1, NaN = not known
struct StrokePoint { x @0 :Float64; y @1 :Float64; pressure @2 :Float32 = nan; }

struct Rect { x @0 :Float64; y @1 :Float64; width @2 :Float64; height @3 :Float64; }

enum ShapeKind { rectangle @0; ellipse @1; diamond @2; line @3; }

# typed properties, not a free string; colours are CSS hex text, empty = none
struct Style {
  stroke     @0 :Text;
  fill       @1 :Text;
  lineWidth  @2 :Float64 = 1;
  dashed     @3 :Bool;
  opacity    @4 :Float32 = 1;   # 0..1
  fontFamily @5 :Text;
  fontSize   @6 :Float32 = 16;
}

struct Element {
  envelope @0 :Envelope;        # store type `WhiteboardElement`; `context` = the board's IRI
  board    @1 :Uuid;
  page     @2 :Uuid;
  parent   @3 :Uuid;            # group or frame; unset = directly on the page
  order    @4 :Text;            # position front to back: sorted as bytes, then by id; a later one is in front
  frame    @5 :Rect;            # position and size before rotation
  angle    @6 :Float64;         # radians, around the middle of `frame`
  style    @7 :Style;
  locked   @8 :Bool;
  deleted  @9 :Bool;
  union {
    shape     @10 :ShapeKind;
    text      @11 :Text;        # free text; plain, no markup assumed
    note      @12 :Text;        # text on a coloured shape
    stroke    @13 :List(StrokePoint);   # at most 100 000 points; a longer line is several strokes
    connector :group {
      source      @14 :Uuid;    # follows that element; unset = free end at `sourcePoint`
      target      @15 :Uuid;
      sourcePoint @16 :Point;
      targetPoint @17 :Point;
      label       @18 :Text;
      arrowEnd    @19 :Bool;
    }
    image     @20 :Text;        # file IRI (files.capnp)
    group     @21 :Void;        # holds the elements whose `parent` it is
    area      @22 :Text;        # a named frame on the board; bounded area
  }
  label    @23 :Text;           # text bound to a shape; other kinds ignore it
}

struct Page { id @0 :Uuid; name @1 :Text; order @2 :Text; deleted @3 :Bool; }

struct Board {
  envelope @0 :Envelope;        # AS2 `Collection`
  as2      @1 :Text;
  pages    @2 :List(Page);      # at least one not deleted; in order
}

struct BoardPage { items @0 :List(Board); next @1 :Cursor; }

# one property of a style; a `setStyle` changes only those it names, so it merges with others' changes
struct StyleField {
  union {
    stroke     @0 :Text;
    fill       @1 :Text;
    lineWidth  @2 :Float64;
    dashed     @3 :Bool;
    opacity    @4 :Float32;
    fontFamily @5 :Text;
    fontSize   @6 :Float32;
  }
}

enum ConnectorEnd { source @0; target @1; }

enum Align { left @0; centerX @1; right @2; top @3; centerY @4; bottom @5; distributeX @6; distributeY @7; }

# root capability; other ids are named at the methods. Not in this contract yet: sync, board.sync,
# board.share, presence.*, library.*, template.*, comment.add; each is reported unsupported.
interface Whiteboard {
  capabilities  @0 () -> (capabilities :CapabilityList);
  supportReport @1 () -> (report :SupportReport);                 # provider rule (ADR 0045)
  boards        @2 () -> (boards :Boards);
  subscribe     @3 (board :Uuid, listener :ChangeListener) -> (subscription :Subscription); # board.subscribe
}

interface Boards {
  create @0 (name :Text) -> (board :Board);                        # board.create
  open   @1 (id :Uuid) -> (found :Bool, board :BoardEditor);       # board.open
  rename @2 (id :Uuid, name :Text) -> (board :Board);              # board.create
  # with every element of it
  delete @3 (id :Uuid) -> ();                                      # board.delete
  list   @4 (cursor :Cursor) -> (page :BoardPage);                 # board.open
  # our own open format, JSON, at most 32 MiB; other formats are Unsupported; keeps a foreign original as it is
  import @5 (mediaType :Text, data :Data) -> (board :Board);       # board.import
}

# An edit of a locked element is refused, other than `setLocked`. All ids must be on this board.
# One editor is one person's session: it starts at `open` and ends when the capability is dropped or
# the connection is lost, and with it ends its undo history.
interface BoardEditor {
  get       @0 () -> (board :Board);                               # board.open
  addPage   @1 (name :Text) -> (board :Board, page :Uuid);         # page.add
  removePage @2 (page :Uuid) -> (board :Board);                    # page.remove; not the last one
  movePage  @3 (page :Uuid, before :Uuid) -> (board :Board);       # page.add; unset `before` = last

  # the id and `order` given are ignored; a new element goes to the front of its page
  add       @4 (element :Element) -> (element :Element);           # shape.add, text.add, note.add, stroke.draw, connector.add, image.add, frame.add
  getElement @5 (id :Uuid) -> (found :Bool, element :Element);      # board.open
  # in order, back to front; deleted ones are left out; unset `area` = the whole page
  elements  @6 (page :Uuid, area :Rect, cursor :Cursor) -> (items :List(Element), next :Cursor); # board.view
  bounds    @7 (page :Uuid) -> (found :Bool, area :Rect);          # canvas.navigate

  move      @8 (ids :List(Uuid), dx :Float64, dy :Float64) -> ();  # element.move; a group moves with its members
  setFrame  @9 (id :Uuid, frame :Rect) -> ();                      # element.move; resize; width and height at least 0
  rotate    @10 (id :Uuid, angle :Float64) -> ();                  # element.move
  setStyle  @11 (ids :List(Uuid), fields :List(StyleField)) -> (); # element.style
  setText   @12 (id :Uuid, text :Text) -> ();                      # text.edit; text, note, area, connector label
  # one end; unset `element` = free at `point`
  setEnd    @13 (id :Uuid, end :ConnectorEnd, element :Uuid, point :Point) -> ();   # connector.edit
  setLocked @14 (ids :List(Uuid), locked :Bool) -> ();             # element.lock
  delete    @15 (ids :List(Uuid)) -> ();                           # element.delete, stroke.erase; a group with its members
  copy      @16 (ids :List(Uuid), dx :Float64, dy :Float64) -> (ids :List(Uuid)); # element.move
  group     @17 (ids :List(Uuid)) -> (id :Uuid);                   # element.group; refused if it would form a cycle
  ungroup   @18 (id :Uuid) -> (ids :List(Uuid));                   # element.ungroup
  # places `ids` directly behind `before`, keeping their order; unset `before` = at the front
  reorder   @19 (ids :List(Uuid), before :Uuid) -> ();             # element.reorder
  align     @20 (ids :List(Uuid), how :Align) -> ();               # element.align

  # undo and redo belong to this editor, so one person's undo does not take back another's work: a
  # property someone else changed since then is skipped
  undo      @21 () -> (done :Bool);                                # history.undo
  redo      @22 () -> (done :Bool);                                # history.redo

  # own format as JSON; image/svg+xml; others are Unsupported. An empty `ids` = the whole page; unset
  # `page` = the whole board, only in the own format. Read in pieces, so size does not end a connection.
  export    @23 (page :Uuid, ids :List(Uuid), mediaType :Text) -> (data :C.Blob); # board.export, board.save
  # to another frame or page; unset `parent` = directly on the page. Refused if it would form a cycle.
  setParent @24 (ids :List(Uuid), page :Uuid, parent :Uuid) -> ();                # element.group
}
