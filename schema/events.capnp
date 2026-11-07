# fs-calendar contract — DRAFT 0.1 (2026-11-06)
#
# STATUS: DRAFT. Owner program: fs-calendar. Lives in fs-interfaces (ADR 0045, 0050).
# Consumers so far: none (0 of 2). Provider: fs-calendar-core.
# JSCalendar `Event` belongs here, `Task` to tasks.capnp; a calendar shows tasks only through that.
# An event travels as JSCalendar 2.0 JSON text, which stays authoritative, next to the AS2 envelope.
# Evolution rules: never renumber or remove fields/methods; only append.

@0x9bc7b40102a75b3f;

using C = import "common.capnp";
using Uuid = C.Uuid;
using Timestamp = C.Timestamp;
using Cursor = C.Cursor;
using Envelope = C.Envelope;
using SupportReport = C.SupportReport;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;

struct Event {
  envelope   @0 :Envelope;         # AS2 `Event` envelope
  jscalendar @1 :Text;             # the JSCalendar 2.0 `Event` as JSON (authoritative content)
  revision   @2 :UInt64;           # changes at every write (for updateIf); compare for equality only, it is no counter
}

struct EventPage { items @0 :List(Event); next @1 :Cursor; }

# one instance of an event; a series is expanded at query time
struct Instance {
  object     @0 :Uuid;
  start      @1 :Timestamp;
  end        @2 :Timestamp;
  jscalendar @3 :Text;             # the instance with its override applied
  overridden @4 :Bool;
}

# Something in an event that could not be read or expanded, or that was cut: the event id, which part, and why.
struct Damaged {
  id     @0 :Uuid;
  reason @1 :Text;                 # for an event cut at a bound: starts with `cut: `
  key    @2 :Text;                 # empty = the whole event; else the key of the damaged exception/instance of a series
}

enum Response { accept @0; tentativeAccept @1; reject @2; }

struct InvitationEntry {
  who      @0 :Text;
  response @1 :Text;               # AS2 activity type name
  at       @2 :Timestamp;
  comment  @3 :Text;
}

struct Invitee { name @0 :Text; email @1 :Text; }

struct ImportReport {
  created @0 :List(Uuid);
  skipped @1 :List(Text);
  failed  @2 :List(Text);
  notes   @3 :List(Text);          # upgrades; tasks left out when no task program is attached
}

interface Events {
  supportReport @0 () -> (report :SupportReport);
  items         @1 () -> (items :Items);
  invitations   @2 () -> (invitations :Invitations);
  subscribe     @3 (listener :ChangeListener) -> (subscription :Subscription);   # event.subscribe
}

interface Items {
  create      @0 (jscalendar :Text) -> (event :Event);                           # event.create
  update      @1 (id :Uuid, jscalendar :Text) -> (event :Event);                 # event.update
  delete      @2 (id :Uuid) -> ();                                               # event.delete
  get         @3 (id :Uuid) -> (found :Bool, event :Event);                      # event.read
  getByUid    @4 (uid :Text) -> (found :Bool, event :Event);                     # event.read
  list        @5 (cursor :Cursor) -> (page :EventPage);                          # event.read
  # `damaged` names what could not be read or expanded (event id, `key`, reason). If only one exception of a series
  # is damaged, its event is in `damaged` AND the rest of that event stays in `items`; a whole event with an empty
  # `key` is missing from `items`.
  # `cut` is true when at least one event was cut at a bound (instances per event, the answer budget), e.g. a series
  # of 8 947 occurrences: `items` is then incomplete instead of the call failing, and `damaged` names the cut events
  # with a reason starting `cut: `. The caller asks again with a narrower range. A 0.42 server leaves both empty/false.
  occurrences @6 (from :Timestamp, to :Timestamp) -> (items :List(Instance), damaged :List(Damaged), cut :Bool);    # event.occurrences
  importText  @7 (mediaType :Text, text :Text) -> (report :ImportReport);        # event.import
  exportText  @8 (ids :List(Uuid), mediaType :Text) -> (text :Text);             # event.export
  updateIf    @9 (id :Uuid, revision :UInt64, jscalendar :Text) -> (event :Event);  # event.update, only if the event still has `revision`; otherwise it fails with a reason starting `conflict:` and writes nothing (compare-and-set; atomic inside one provider process only)
}

interface Invitations {
  invite  @0 (id :Uuid, invitees :List(Invitee)) -> (event :Event);              # invite.create
  respond @1 (id :Uuid, who :Text, response :Response, comment :Text) -> (event :Event);  # invite.answer
  history @2 (id :Uuid) -> (items :List(InvitationEntry));                       # invite.read
}
