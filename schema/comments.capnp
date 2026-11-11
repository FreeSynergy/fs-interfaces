# fs-comments contract — DRAFT 0.1 (2026-11-11)
#
# STATUS: DRAFT, PROVISIONAL (ADR 0045: a draft until two independent users have tested it, then 1.0).
# Owner of the topic: not settled yet (probably the Messenger). First provider: fs-tasks; the wiki will use the same topic.
# Consumers so far: none (0 of 2).
#
# A comment is an AS2 `Note` (ADR 0020) about another object. It is its own object, never a member of the object it is about:
#   - `envelope.context`      = the IRI of the commented object (a task, a wiki block ...);
#   - `envelope.inReplyTo`    = the IRI of the comment it answers; empty = a comment on the object itself;
#   - `envelope.attributedTo` = the author as an **actor IRI**. A reference to a person of fs-people is only resolved for
#     display and never stored. The provider sets the author from the caller's identity; the caller cannot name one.
#   - deleting = AS2 `Tombstone`: the object stays with `tombstone = true` and an empty `body`, so that a thread does not break.
# Threads are one level deep: `inReplyTo` must name a comment that itself has no `inReplyTo`, on the same `context`; otherwise the
# call fails with the problem `invalid-reply` (RFC 9457, status 422). The Web Annotation model (a selector for a place in a text)
# may be added later; nothing here builds it.
#
# Provider rule (ADR 0045): every provider offers all methods; what truly cannot be done answers `unsupportedProblemType`.
# Capability ids: comment.list, comment.add, comment.edit, comment.delete, comment.subscribe.
# A comment of a sensitive object (ADR 0029) inherits the mark; the provider refuses list and export like for the object.
#
# Evolution rules (Cap'n Proto): never renumber or remove fields/methods; only append.

@0xeef17f58a86f6efa;

using C = import "common.capnp";
using Uuid = C.Uuid;
using Timestamp = C.Timestamp;
using Cursor = C.Cursor;
using Envelope = C.Envelope;
using CapabilityList = C.CapabilityList;
using SupportReport = C.SupportReport;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;

struct Comment {
  envelope  @0 :Envelope;         # AS2 `Note` (`Tombstone` once deleted); see above for the members used
  body      @1 :Text;             # plain text; empty once deleted
  tombstone @2 :Bool;
  deleted   @3 :Timestamp;        # when it was deleted; only meaningful if `tombstone`
  revision  @4 :UInt64;           # changes at every write (for editIf); compare for equality only
}

# one page of the comments of one context, oldest first; a reply follows the comment it answers
struct CommentPage { items @0 :List(Comment); next @1 :Cursor; }

interface Comments {
  capabilities  @0 () -> (capabilities :CapabilityList);
  supportReport @1 () -> (report :SupportReport);
  list          @2 (context :Text, cursor :Cursor) -> (page :CommentPage);                       # comment.list
  # `inReplyTo` empty = a comment on the context itself
  add           @3 (context :Text, body :Text, inReplyTo :Text) -> (comment :Comment);           # comment.add
  # Only the author; only if the comment still has `revision`, else it fails with a reason starting `conflict:`.
  edit          @4 (id :Uuid, revision :UInt64, body :Text) -> (comment :Comment);               # comment.edit
  # The author, or the owner of the commented object; leaves a tombstone.
  delete        @5 (id :Uuid) -> (comment :Comment);                                             # comment.delete
  subscribe     @6 (context :Text, listener :ChangeListener) -> (subscription :Subscription);   # comment.subscribe
}
