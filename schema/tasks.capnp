# fs-tasks contract — DRAFT 0.1 (2026-11-06)
#
# STATUS: DRAFT. Owner program: fs-tasks. Lives in fs-interfaces (ADR 0045).
#
# Target rules (ADR 0045): the interface is a target, not a snapshot. It stays DRAFT until two
# independent consumers have tested it, then it becomes 1.0, and from then on only version jumps.
# Consumers so far: fs-calendar (1 of 2).
#
# Source: docs/en/programs/apps/tasks/design.md (interface sketch). The Rust form of `Items` and of
# the subscription is the trait pair `TaskItems` / `TaskFeed` in crates/fs-tasks-core (ADR 0043:
# trait first). A task travels as its JSCalendar 2.0 JSON text, which stays authoritative
# (the standards are JSON; ADR 0014, 0050), next to the AS2 envelope.
#
# Evolution rules (Cap'n Proto): never renumber or remove fields/methods; only append.
# Compiled with common.capnp by capnp-compiler (see crates/fs-interfaces/crates/regenerate).

@0xc04d258971543476;

using C = import "common.capnp";
using Uuid = C.Uuid;
using Timestamp = C.Timestamp;
using Cursor = C.Cursor;
using Envelope = C.Envelope;
using CapabilityList = C.CapabilityList;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;
using T = import "types.capnp";

# RFC 9253 relation of a task to another object (JSCalendar `relatedTo`)
enum Relation {
  first          @0;
  next           @1;
  child          @2;
  parent         @3;
  dependsOn      @4;
  finishToStart  @5;
  finishToFinish @6;
  startToStart   @7;
  startToFinish  @8;
}

# Whether a task can be started, from its dependencies (0.0.19; plan of fs-tasks, ADR 0073). Clients must tolerate values they
# do not know. `ReadinessReport.by` says which tasks the state is about.
enum Readiness {
  ready               @0;   # nothing blocks it
  blocked             @1;   # `by` = uids of the unfinished tasks it waits for
  done                @2;   # progress is completed
  unknown             @3;   # `by` = uids a dependency names that this store does not hold (shown, never guessed)
  dependencyCancelled @4;   # `by` = uids of dependencies that are cancelled or failed: a person decides, nothing frees it by itself
  conflict            @5;   # `by` = the uids of a dependency cycle the task is part of (detected on read, also after a merge)
}

struct ReadinessReport { readiness @0 :Readiness; by @1 :List(Text); }

struct Task {
  envelope   @0 :Envelope;        # AS2 `Task` envelope (id, iri, name, context "tasks", times)
  jscalendar @1 :Text;            # the JSCalendar 2.0 `Task` as JSON (authoritative content)
  revision   @2 :UInt64;          # changes at every write (for editIf); compare for equality only, it is no counter
}

# one page of tasks, ordered by due date (undated last)
struct TaskPage { items @0 :List(Task); next @1 :Cursor; }

# all members optional; unset (null / empty / false) = no restriction; combined with AND.
# The filter applies to `query` on every page: the cursor continues the same filtered, ordered list, so
# the caller sends the same filter with each cursor (`list` takes no filter and returns everything).
# "Open" = not finished: needs-action and in-process. completed, failed and cancelled are finished.
# "Done" is `progress = "completed"`; `open` and `progress` are both applied (AND), so `open = true`
# together with a finished `progress` matches nothing.
struct TaskFilter {
  dueFrom  @0 :Timestamp;         # due at or after
  dueTo    @1 :Timestamp;         # due before (half-open range)
  progress @2 :Text;              # needs-action | in-process | completed | failed | cancelled
  label    @3 :Text;              # JSCalendar keyword
  term     @4 :Text;              # substring of title and description; case-insensitive; empty = no restriction
  open     @5 :Bool;              # true = only open tasks: progress needs-action or in-process (a task without progress counts as needs-action); false = no restriction
}

struct ImportReport {
  created @0 :List(Uuid);
  skipped @1 :List(Text);         # uids that already existed
  failed  @2 :List(Text);
  notes   @3 :List(Text);
}

# root capability (capability ids: task.create, task.edit, task.assign, relation.create, label.assign,
# task.read, task.query, task.delete, task.import, task.export, task.subscribe; boards: board.*, later)
interface Tasks {
  capabilities @0 () -> (capabilities :CapabilityList);
  items        @1 () -> (items :Items);
  subscribe    @2 (listener :ChangeListener) -> (subscription :Subscription);
  boards       @3 () -> (boards :Boards);      # placeholder, see below
  supportReport @4 () -> (report :C.SupportReport);  # provider rule (ADR 0045)
  # The data types this program owns (`Task`) and what can be done with them (ADR 0066), for a generated frontend.
  types         @5 () -> (catalog :T.TypeCatalog);
}

interface Items {
  create         @0  (jscalendar :Text) -> (task :Task);                                   # task.create
  edit           @1  (id :Uuid, jscalendar :Text) -> (task :Task);                         # task.edit
  assign         @2  (id :Uuid, name :Text, email :Text) -> (task :Task);                  # task.assign
  createRelation @3  (id :Uuid, otherUid :Text, relation :Relation) -> (task :Task);       # relation.create
  assignLabel    @4  (id :Uuid, label :Text) -> (task :Task);                              # label.assign
  get            @5  (id :Uuid) -> (found :Bool, task :Task);
  getByUid       @6  (uid :Text) -> (found :Bool, task :Task);
  delete         @7  (id :Uuid) -> ();
  list           @8  (cursor :Cursor) -> (page :TaskPage);
  query          @9  (filter :TaskFilter, cursor :Cursor) -> (page :TaskPage);             # time range, status, label
  importIcs      @10 (text :Text) -> (report :ImportReport);                               # VTODO
  exportIcs      @11 (ids :List(Uuid)) -> (text :Text);
  editIf         @12 (id :Uuid, revision :UInt64, jscalendar :Text) -> (task :Task);       # task.edit, only if the task still has `revision`; otherwise it fails with a reason starting `conflict:` and writes nothing (compare-and-set; atomic inside one provider process only)
  # Appended in 0.0.19. Capability ids `dependency.evaluate` (readiness, blockedBy, dependents) and `dependency.cycle-check`.
  readiness      @13 (id :Uuid) -> (report :ReadinessReport);                              # dependency.evaluate
  blockedBy      @14 (id :Uuid) -> (tasks :List(Task));                                    # dependency.evaluate; the unfinished tasks it waits for
  dependents     @15 (id :Uuid) -> (tasks :List(Task));                                    # dependency.evaluate; the tasks that wait for it (reverse edges)
  # Like `createRelation`, with the lag (positive) or lead (negative) of a temporal relation: an ISO 8601 duration such as
  # `P1D` or `-PT4H`, stored as the member `freesynergy.net:gap` of the JSCalendar `Relation` and written as RFC 9253 `GAP`.
  # Empty = none. A gap on a relation without temporal meaning is refused. `createRelation` stays as it was.
  createRelationWithGap @16 (id :Uuid, otherUid :Text, relation :Relation, gap :Text) -> (task :Task);   # relation.create
  # A relation that would close a dependency cycle is refused by `createRelation*`: the problem type is `cycle` (status 409),
  # its extension `path` repeats once for every uid of the cycle, in order.
}

# PLACEHOLDER — declared so the root can hand it out; NOT implemented and without methods yet.
# Planned (design.md): board.open, board.create, column.create, column.limit, card.move.
# Moving a card is an `Update` on `task_link`, never on the task (projects/technical.md).
interface Boards {
}
