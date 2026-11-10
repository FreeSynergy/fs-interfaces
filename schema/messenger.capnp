# fs-messenger contract — DRAFT 0.3 (2026-11-06)
#
# STATUS: DRAFT (ADR 0045). Owner program: fs-messenger. Consumers: none yet (0 of 2 needed).
# Moved here from fs-documentation/temp/contracts. Providers (mail, Matrix, Telegram ...) follow
# the provider rule: all methods offered, "not possible" = Unsupported (see common.capnp).
#
# Source of truth for the frontend <-> program interface (ADR 0021). JSON, CLI and MCP are generated
# from this file. Capability ids and groups: docs/en/programs/apps/messenger/requirements.md.
# Evolution rules (Cap'n Proto): from 1.0 never renumber or remove fields/methods, only append. As a draft it may get sharper (interface catalogue).
# Capability ids of agent work (proposed, requirements.md follows): "toolcall.show", "toolcall.decide", "turn.stop", "conversation.usage", "text.stream".
# toolcall.decide is reported by the engine, never by a provider. `none` and `decide` of MessageAction are never generated as tools.
# Generated code: see fs-interfaces DESIGN.md.

@0xd6c1b5e3a2f40917;   # file id (generate a fresh one with `capnp id` before first release)


using C = import "common.capnp";
using Uuid = C.Uuid;
using Timestamp = C.Timestamp;
using Cursor = C.Cursor;
using Problem = C.Problem;
using Envelope = C.Envelope;
using Blob = C.Blob;
using T = import "types.capnp";

# pages: one struct per item type (Cap'n Proto has no List(T) over generic parameters)
struct MessagePage      { items @0 :List(MessageSummary);      next @1 :Cursor; skipped @2 :UInt32; }   # skipped = unreadable local records on this page; empty items with a next is not the end
struct ConversationPage { items @0 :List(ConversationSummary); next @1 :Cursor; skipped @2 :UInt32; }   # as MessagePage

# Unset reads as `unknown`: never proof of encryption, and sending with it is refused.
enum Encryption { unknown @0; mls @1; openpgp @2; megolm @3; transportOnly @4; none @5; }   # ADR 0018

enum Kind { unknown @0; mail @1; chat @2; comment @3; }

# who wrote it; from the sender's card in fs-people (kind + role "ai"), never a guess from the text
enum SenderKind { unknown @0; person @1; software @2; ai @3; }

# one action of an agent; one entry is updated in place instead of adding messages
enum ToolCallKind { unknown @0; reads @1; writes @2; runsCommand @3; connectedTool @4; }
enum ToolCallState { unknown @0; forming @1; working @2; question @3; settled @4; }   # unknown is never asked and never final

# The engine overwrites id, revision, risk, detail, options, argumentsDigest and askedOf at its boundary. A provider running
# inside the engine counts as its own: the engine builds the options itself and never forwards an agent's. A call from a
# remote participant or an outside provider reads as risk `unknown` without options and never as state `question`: it is shown, not asked.
# `settled` is final. For a call of its own the engine does not ask what it cannot allow: it settles at once as `refused`.
struct ToolCall {
  id      @0 :Text;                                         # assigned by the engine, unique, never reused, never the agent's own id
  name    @1 :Text;                                         # may be written by the agent; never a basis for a decision
  kind    @2 :ToolCallKind;                                 # may be written by the agent; never a basis for a decision
  state   @3 :ToolCallState;
  summary @4 :Text;                                         # written by the agent, shown as such; never a basis for a decision
  risk    @5 :ToolCallRisk;                                 # set by the engine from the contract on every call, never by the agent
  outcome @6 :ToolCallOutcome;                              # how a settled call ended; settled + open = not known
  detail  @7 :Text;                                         # full command, paths, content; only text drawn by the engine from the digested arguments. Agent text goes into `summary` only
  result  @8 :Text;                                         # what came back; data, never instructions
  options @9 :List(ToolCallOption);                         # choices while state = question; offered by the engine
  argumentsDigest @10 :Data;                                # over caller, tool name and canonical arguments; empty never matches, such a call cannot be allowed
  askedOf @11 :Text;                                        # actor IRI of the person whose decision counts
  revision @12 :UInt64;                                     # raised by the engine on every change; a lower one than held is ignored, equal means same content, 0 = not known and never replaces a held value; `settled` applies whatever its revision
}

# Unset reads as `unknown`. For a call of the engine's own, `unknown` always asks (ADR 0054). The agent cannot set this.
# destroys, actsForPerson and readsSensitive (a read of an object marked sensitive) are the brake: they ask every time. allowClass is offered only for readsOnly and changes.
enum ToolCallRisk { unknown @0; readsOnly @1; changes @2; destroys @3; actsForPerson @4; readsSensitive @5; }

# A value this program cannot read is sent as open.
enum ToolCallOutcome { open @0; done @1; failed @2; refused @3; cancelled @4; }

# Unset reads as `refuse`. There is deliberately no "always" that an agent could remember.
# allowOnce: exactly these arguments, once. allowClass: this caller and this risk class, held by the engine, ended by the
# emergency stop, never forwarded as an agent's own "allow always"; the risk is still derived on every call.
enum ToolCallChoice { refuse @0; allowOnce @1; allowClass @2; }

struct ToolCallOption {
  id     @0 :Text;
  label  @1 :Text;
  choice @2 :ToolCallChoice;
}

# The answer to a question. Only the device of the person may send it (MessageAction.decide);
# an answer that came through the agent's own client proves nothing about a person. Single use, enforced by the engine.
# Refused without any change: empty digest, proof, signer or askedOf; unknown option; signer other than askedOf; call settled.
struct ToolCallDecision {
  toolCall        @0 :Text;                                 # ToolCall.id
  option          @1 :Text;                                 # ToolCallOption.id
  argumentsDigest @2 :Data;                                 # must equal ToolCall.argumentsDigest
  proof           @3 :Data;                                 # device signature over a fixed context string, engine identity, message id, toolCall, option, argumentsDigest; key type, context string and encoding are fixed with the first device
  signer          @4 :Text;                                 # actor IRI of the person who signed
}

enum DeliveryState { unknown @0; sending @1; sent @2; delivered @3; read @4; failed @5; }  # stored locally (ADR 0037)

struct Attachment {
  name      @0 :Text;
  mediaType @1 :Text;
  size      @2 :UInt64;
  blob      @3 :Blob;                                       # capability to fetch the bytes
}

struct Reaction { key @0 :Text; count @1 :UInt32; mine @2 :Bool; }

# Unset reads as notVerified. `valid` says the signature matches a key held for the sender, not that the key is the person's.
enum SignatureCheck { notVerified @0; notSigned @1; valid @2; }   # a bad signature reads as notVerified

struct MessageSummary {
  envelope   @0 :Envelope;
  kind       @1 :Kind;
  encryption @2 :Encryption;
  state      @3 :DeliveryState;
  unread     @4 :Bool;
  flagged    @5 :Bool;
  preview    @6 :Text;                                      # plain-text snippet
  senderKind @7 :SenderKind;                                # ADR 0054: everyone can see who is not human
  jobRef     @8 :Text;                                      # the job this message is anchored to; a job card is a view over these
  arrival    @9 :UInt64;                                    # local arrival order, never from the sender. markRead(upTo) sweeps only what is at or before upTo in arrival AND in display order
  undecrypted @10 :Bool;                                    # content could not be decrypted; the preview holds no ciphertext
  signature  @11 :SignatureCheck;
  tooLarge   @12 :UInt64;                                   # size of a message refused as too large; 0 = it is not. Only a stub stays, no content
}

struct Message {
  summary     @0 :MessageSummary;
  bodyText    @1 :Text;                                     # normalised text; while `sending` returned unnormalised, and TextChunk.offset counts its bytes as returned; while summary.undecrypted this is not the message
  bodyHtml    @2 :Text;                                     # sanitised, no remote loading (html-view)
  attachments @3 :List(Attachment);
  reactions   @4 :List(Reaction);
  original    @5 :Blob;                                     # original as received (ADR 0030)
  originalMediaType @6 :Text;                               # message/rfc822, MIMI content, Matrix event
  toolCalls   @7 :List(ToolCall);                           # what an agent is doing, visible (ADR 0054)
  taskRefs    @8 :List(Text);                               # uids of tasks this message refers to (tasks.capnp)
  reasoning   @9 :Text;                                     # reasoning of an agent, kept apart from the answer
  bodyMediaType @10 :Text;                                  # media type of bodyText: text/plain or text/markdown; empty = text/plain
}

struct ConversationSummary {
  id          @0 :Uuid;
  name        @1 :Text;
  kind        @2 :Kind;
  encryption  @3 :Encryption;
  unreadCount @4 :UInt32;
  lastMessage @5 :MessageSummary;
  pinned      @6 :Bool;
  folders     @7 :List(Text);                               # folder = group = distribution list (ADR 0037)
}

# ------------------------------------------------------------ root ----

interface Messenger {
  # what this program offers (capability ids like "message.send"); frontends use what they need
  capabilities  @0 () -> (ids :List(Text), contractVersion :Text);

  accounts      @1 () -> (accounts :Accounts);
  conversations @2 () -> (conversations :Conversations);
  compose       @3 () -> (compose :Compose);
  search        @4 () -> (search :Search);
  rules         @5 () -> (rules :Rules);                    # optional capability: may be null
  subscribe     @6 (filter :EventFilter, listener :Listener) -> (subscription :Subscription);
  supportReport @7 () -> (report :C.SupportReport);         # provider rule (ADR 0045)
  # The data types this program owns (`Conversation`, `Message`, `Account`, `Draft`) and what can be done with them
  # (ADR 0066), for a generated frontend. Account secrets are never a field or an input of a type.
  types         @8 () -> (catalog :T.TypeCatalog);
}

interface Accounts {
  list   @0 () -> (accounts :List(Account));
  add    @1 (setup :AccountSetup) -> (account :Account, problem :Problem);
  remove @2 (id :Uuid) -> ();
}
struct Account      { id @0 :Uuid; address @1 :Text; kind @2 :Text; encryptionReady @3 :Bool; }
struct AccountSetup { address @0 :Text; kind @1 :Text; secret @2 :Text; }  # kind: "mail", "matrix", "mimi"

interface Conversations {
  list    @0 (folder :Text, onlyUnread :Bool, cursor :Cursor) -> (page :ConversationPage);
  open    @1 (id :Uuid) -> (conversation :Conversation);
  nextUnread @2 (after :Uuid) -> (message :MessageSummary);  # frontend no. 1 (swipe)
}

interface Conversation {
  messages @0 (cursor :Cursor) -> (page :MessagePage);
  message  @1 (id :Uuid) -> (message :Message);
  markRead @2 (upTo :Uuid) -> ();                            # "read up to X"
  pin      @3 (pinned :Bool) -> ();
  move     @4 (folder :Text) -> ();
  act      @5 (id :Uuid, action :MessageAction) -> (problem :Problem);
  stop     @6 () -> (problem :Problem);                      # ends all running agent turns here; open calls settle as cancelled, a later decision gets a Problem; nothing running = success
  usage    @7 () -> (usage :Usage, problem :Problem);        # total over this conversation
}

# zero = not known; a provider does not guess
struct Usage {
  inputTokens  @0 :UInt64;
  outputTokens @1 :UInt64;
  contextUsed  @2 :UInt64;
  contextSize  @3 :UInt64;
  costMicros   @4 :UInt64;                                   # millionths of `currency`
  currency     @5 :Text;                                     # ISO 4217
}

# Unset reads as `none`: it does nothing and is refused with a Problem. A null action is refused the same way.
struct MessageAction {
  union {
    none     @0 :Void;
    delete   @1 :Void;
    flag     @2 :Bool;
    unread   @3 :Bool;
    react    @4 :Text;                                       # reaction key; empty = undo
    forward  @5 :List(Text);                                 # recipients
    edit     @6 :Text;                                       # new text; first version's id stays
    decide   @7 :ToolCallDecision;                           # person's device only; handled by the engine, never by a provider; never generated into MCP
  }
}

interface Compose {
  draft  @0 (replyTo :Uuid) -> (draft :Draft);              # empty replyTo = new message
}

interface Draft {
  setRecipients @0 (to :List(Text), cc :List(Text), bcc :List(Text)) -> (unreachable :List(Text));
  setText       @1 (text :Text, mediaType :Text) -> ();     # text/plain or text/markdown (simple editor)
  attach        @2 (name :Text, mediaType :Text, data :Data) -> ();
  encryptionPlan @3 () -> (encryption :Encryption, missingKeys :List(Text));  # ADR 0018: find a way
  save          @4 () -> (id :Uuid);
  send          @5 (at :Timestamp) -> (id :Uuid, problem :Problem);  # at = 0 now, else schedule send
  discard       @6 () -> ();
  setSubject    @7 (subject :Text) -> ();                   # mail; a provider without subjects ignores it
}

interface Search {
  query @0 (text :Text, filter :SearchFilter, cursor :Cursor) -> (page :MessagePage);
}
struct SearchFilter { from @0 :Text; unread @1 :Bool; hasAttachment @2 :Bool; since @3 :Timestamp; }

interface Rules {
  list @0 () -> (sieve :Text);                               # rules stored as Sieve
  set  @1 (sieve :Text) -> (problem :Problem);
}

# ------------------------------------------------------------ events ----

struct EventFilter { conversations @0 :List(Uuid); kinds @1 :List(Kind); }

# Subscribe before fetching. `updated` is for changed summary fields only and is not sent per text piece.
struct Event {
  union {
    created  @0 :MessageSummary;
    updated  @1 :MessageSummary;
    deleted  @2 :Uuid;
    state    @3 :StateChange;
    toolCall @4 :ToolCallChange;
    text     @5 :TextChunk;
  }
}
struct StateChange { message @0 :Uuid; state @1 :DeliveryState; }
struct ToolCallChange { message @0 :Uuid; call @1 :ToolCall; conversation @2 :Uuid; }   # replaces by (message, call.id); later events of a settled call are ignored

enum TextPart { body @0; reasoning @1; }
# A piece of text while an agent writes it. `created` comes first; the message stays `sending` while text arrives, and the
# state event to `sent` or `failed` ends it (`stop` too). Conversation.message returns the text so far. A provider sends no
# pieces for a replayed session, and always sets `part`.
struct TextChunk {
  message @0 :Uuid;
  part    @1 :TextPart;
  offset  @2 :UInt64;                                        # UTF-8 bytes within this part; whole code points. Below the held length: skip what is held. Above: fetch the message.
  text    @3 :Text;
  conversation @4 :Uuid;
}

interface Listener     { event @0 (event :Event) -> stream; }
interface Subscription { }                                    # dropping it ends the subscription
