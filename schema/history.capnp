# fs-history contract — DRAFT 0.1 (2026-11-06)
#
# STATUS: DRAFT, PROVISIONAL. Owner program: fs-history. Lives in fs-interfaces (ADR 0045).
# Consumers so far: none (0 of 2). No history frontends are analysed yet.
#
# An observer of what happened, in every program (ADR 0020). It writes nothing of its own but a
# derived index of entries: a summary of each activity the hub delivers, never the content. Sensitive
# objects never reach it, because the hub withholds them (ADR 0029), and it never reads `Relationship`.
# The index is local and belongs to the person. What a person forgets stays forgotten in this index,
# also when the hub delivers it again; the hub's own log is not touched. When an object is deleted, or
# becomes sensitive, which the hub reports the same way, its entries keep no name and no context.
#
# Evolution rules (Cap'n Proto): never renumber or remove fields/methods; only append.

@0xe81c7a35d94b2f06;

using C = import "common.capnp";
using Uuid = C.Uuid;
using Timestamp = C.Timestamp;
using Cursor = C.Cursor;
using CapabilityList = C.CapabilityList;
using SupportReport = C.SupportReport;
using Subscription = C.Subscription;

struct Entry {
  id         @0 :Uuid;        # the activity id in the hub log
  at         @1 :Timestamp;   # when it happened if the hub knows that, else when the hub received it
  program    @2 :Text;        # which program it happened in
  kind       @3 :Text;        # AS2 verb as the hub logs it: Create, Update, Delete, Invite, ...
  object     @4 :Uuid;
  objectType @5 :Text;        # AS2 type
  name       @6 :Text;        # derived display name at that moment; for log-only verbs the last known
  attributedTo @7 :Text;      # who the object belongs to; not always who acted
  context    @8 :Text;        # the object's context, for a project view; for log-only verbs the last known
  actor      @9 :Text;        # who acted; empty until the hub delivers it
  atIsReceived @10 :Bool;     # `at` is only when the hub received it, so it can be much later
}

struct EntryPage { items @0 :List(Entry); next @1 :Cursor; }

# all members optional; unset (null pointer, empty text) = no restriction; combined with AND.
# An all-zero `Uuid` or `Timestamp` counts as unset as well.
struct EntryFilter {
  from       @0 :Timestamp;   # at or after
  to         @1 :Timestamp;   # before (half-open range)
  program    @2 :Text;
  person     @3 :Text;        # matches `actor`, or `attributedTo` where `actor` is empty
  objectType @4 :Text;
  context    @5 :Text;
  object     @6 :Uuid;
}

# live view of the index: only what the filter of the subscription lets through
interface EntryListener {
  added     @0 (entry :Entry) -> stream;
  forgotten @1 () -> stream;  # entries were removed or blanked; read again
}

# what a log has too much of: changes to one object by one program, close together, as one line
struct DigestEntry {
  first  @0 :Entry;           # the earliest entry of the group
  lastAt @1 :Timestamp;       # the latest
  count  @2 :UInt32;
  kinds  @3 :List(Text);      # each verb once, in order of first appearance
  continues @4 :Bool;         # cut off here because the run is very long; the next page goes on with it
}

struct DigestPage { items @0 :List(DigestEntry); next @1 :Cursor; }

# root capability; capability ids: history.observe, history.read, history.digest, history.forget, history.subscribe
interface History {
  capabilities  @0 () -> (capabilities :CapabilityList);
  supportReport @1 () -> (report :SupportReport);                                  # provider rule (ADR 0045)
  # newest first
  timeline @2 (filter :EntryFilter, cursor :Cursor) -> (page :EntryPage);          # history.read
  get      @3 (id :Uuid) -> (found :Bool, entry :Entry);                           # history.read
  # entries of one object and program, no more than `windowMinutes` apart, become one; newest first
  digest   @4 (filter :EntryFilter, windowMinutes :UInt32, cursor :Cursor) -> (page :DigestPage); # history.digest
  # removes the matching entries, past ones only: what happens later is recorded again. They stay out
  # if delivered again. An empty filter is refused
  forget   @5 (filter :EntryFilter) -> (removed :UInt32);                          # history.forget
  subscribe @6 (filter :EntryFilter, listener :EntryListener) -> (subscription :Subscription); # history.subscribe
}
