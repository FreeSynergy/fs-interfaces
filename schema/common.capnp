# FreeSynergy common contract types — DRAFT 0.2 (2026-11-06)
#
# STATUS: DRAFT (ADR 0045: draft -> tested by >= 2 independent consumers -> 1.0 -> only version jumps).
# Owner program: fs-store (envelope, activity log; proposed in ADR 0050) - shared types for all topics.
# Consumers: fs-tasks, fs-hub, fs-messenger (contract draft), fs-calendar.
#
# Shared by every program contract (ADR 0021). Evolution rules: only append, never renumber.
# Provider rule (ADR 0045): every provider offers ALL methods of its interface; what truly cannot be
# done answers explicitly "not possible" = `Problem` of type `unsupportedProblemType`, and each
# provider reports what it can do as a list of `Support`.
@0xb3f1c2d4e5a69788;

struct Uuid      { hi @0 :UInt64; lo @1 :UInt64; }          # UUIDv7 (ADR 0027)
struct Timestamp { unixMillis @0 :Int64; }
# Pagination; empty = start / end. An item too large for any message cannot be delivered: the call
# answers the problem `too-large`, and its error text carries a cursor past the item (in Rust
# `rpc::Problem::next`, read by `Problem::from_error`; none if nothing is behind it). A consumer that hands
# it back skips that item and reads on. A smaller item above the usual page size comes alone on its page.
struct Cursor    { opaque @0 :Data; }
# One RFC 9457 extension member. A key may repeat (a list, e.g. several `issue`).
struct Extension { key @0 :Text; value @1 :Text; }
struct Problem   {                                          # RFC 9457 shape
  type   @0 :Text;
  title  @1 :Text;
  detail @2 :Text;
  status @3 :UInt16;
  # In Rust (`rpc::Problem::extensions`, `Problem::with_ext`) the extensions travel in the error
  # text like `next` and in this member too; at most 64 entries, keys of at most 64 bytes, all
  # texts together at most 64 KiB, an entry that does not fit is left out whole.
  extensions @4 :List(Extension);
}

interface Blob {                                            # bytes behind a capability (originals, attachments)
  size @0 () -> (bytes :UInt64, mediaType :Text);
  read @1 (offset :UInt64, length :UInt32) -> (data :Data);  # may answer fewer bytes than asked; empty = the end
}

# AS2 envelope (ADR 0020, 0032): every object carries it
struct Envelope {
  id           @0 :Uuid;
  iri          @1 :Text;                                    # https://<instance>/<domain>/<uuid>
  type         @2 :Text;                                    # AS2 type: Profile, Event, Note, Article, Collection …
  name         @3 :Text;                                    # derived display name
  attributedTo @4 :Text;                                    # actor IRI / DID
  to           @5 :List(Text);
  cc           @6 :List(Text);
  bto          @7 :List(Text);
  bcc          @8 :List(Text);
  audience     @9 :List(Text);
  context      @10 :Text;
  inReplyTo    @11 :Text;
  published    @12 :Timestamp;
  updated      @13 :Timestamp;
  sensitive    @14 :List(Text);                             # DPV categories (ADR 0029)
  describes    @15 :Text;                                   # AS2 Profile: whom this describes (actor IRI / DID)
  startTime    @16 :Timestamp;                              # AS2 Event
  endTime      @17 :Timestamp;                              # AS2 Event
}

# what a program offers (ADR 0021): capability ids like "message.send", "contact.create"
struct CapabilityList {
  contractVersion @0 :Text;
  ids             @1 :List(Text);
}

# live updates, shared shape for all programs
struct Change {
  union {
    created @0 :Envelope;
    updated @1 :Envelope;
    deleted @2 :Uuid;
  }
}
# `ended` is the last call of a feed that ended without the consumer dropping its `Subscription`: the
# consumer fell too far behind, a change could not be told, or the source closed. Changes may have been
# missed. To go on, subscribe again and then read the current state (`list`, `query`), in that order.
# A listener must not answer `change` with an error: a stream that failed once stays failed, later calls
# on it fail too, and so a listener that failed or stopped answering cannot be told by `ended`. A
# consumer that must know watches its connection as well.
interface ChangeListener {
  change @0 (change :Change) -> stream;
  ended  @1 (problem :Problem) -> ();
}
interface Subscription { }                                  # dropping it ends the subscription

# ---- Provider rule (ADR 0045): like a parent class, every provider offers all methods ----

# Problem type URI of "not possible": a provider that, after real effort, cannot do a method
# answers with a `Problem` of this type (status 501) and the capability id in `detail`.
const unsupportedProblemType :Text = "https://freesynergy.net/problems/unsupported";

# What one provider can do for one capability id ("message.send", "task.importIcs" ...).
# `supported = false` is the explicit "not possible"; `reason` says why (and what was tried).
struct Support {
  capability @0 :Text;
  supported  @1 :Bool;
  reason     @2 :Text;
}

# The "not possible" answer in one value: which capability, and the RFC 9457 problem to return.
struct Unsupported {
  capability @0 :Text;
  problem    @1 :Problem;
}

# A provider's full report: one `Support` per capability of the interface it implements.
struct SupportReport {
  contractVersion @0 :Text;
  provider        @1 :Text;
  entries         @2 :List(Support);
  # Identity of the provider's store (`fs-store` `StoreId`), made once when the store is created.
  # Empty = not known, e.g. a provider without a store. The id is no secret, but it makes the server
  # recognisable: it does not travel to the hub or into content unless someone decided so.
  instance        @3 :Uuid;
}
