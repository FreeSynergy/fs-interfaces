# fs-streaming contract — DRAFT 0.1 (2026-11-06)
#
# STATUS: DRAFT, PROVISIONAL. Owner program: fs-streaming. Lives in fs-interfaces (ADR 0045).
# Consumers so far: none (0 of 2). Stage 1 of docs/en/programs/apps/streaming/design.md: receive one
# stream, send it to many targets without decoding it, and record it. Nothing here decodes or encodes,
# so any codec a sender uses passes through (enhanced RTMP included).
#
# A live stream is not a file: recordings land in fs-files and are listed there (ADR 0050). Only the
# handing over is here.
#
# Stream keys are secrets. A publisher key is shown once, when it is made, and only its hash is kept.
# A target's key is written once and never read back. Neither reaches an activity log, `subscribe`, an
# error text or the hub.
#
# Evolution rules (Cap'n Proto): never renumber or remove fields/methods; only append.

@0xa3d58e0c7b16f924;

using C = import "common.capnp";
using Uuid = C.Uuid;
using Timestamp = C.Timestamp;
using Cursor = C.Cursor;
using Envelope = C.Envelope;
using CapabilityList = C.CapabilityList;
using SupportReport = C.SupportReport;
using Subscription = C.Subscription;

enum StreamState { idle @0; checking @1; live @2; ended @3; error @4; }

struct Stream {
  envelope  @0 :Envelope;       # store type `LiveStream`
  state     @1 :StreamState;
  recording @2 :Bool;           # records when it goes live, and while it is
  note      @3 :Text;           # why, for `error`
}

struct StreamPage { items @0 :List(Stream); next @1 :Cursor; }

enum TargetState {
  off        @0;                # switched off
  waiting    @1;                # the stream is not live
  connecting @2;
  sending    @3;
  retrying   @4;                # lost; tries again, with growing pauses
  refused    @5;                # the far end said no, for example a wrong key; no more tries until changed
}

struct Target {
  id       @0 :Uuid;
  stream   @1 :Uuid;
  name     @2 :Text;
  url      @3 :Text;            # rtmp:// or rtmps://, host and application, never the key
  hasKey   @4 :Bool;
  enabled  @5 :Bool;
  secure   @6 :Bool;            # rtmps; a plain rtmp target travels readable, the key included
  state    @7 :TargetState;
  note     @8 :Text;            # the last problem, with no key in it
}

# what a running stream is doing now; nothing of this is stored
struct Status {
  state   @0 :StreamState;
  since   @1 :Timestamp;
  bytesIn @2 :UInt64;
  targets @3 :List(TargetStatus);
}

struct TargetStatus {
  target    @0 :Uuid;
  state     @1 :TargetState;
  bytesOut  @2 :UInt64;
  dropped   @3 :UInt64;         # frames left out for this target because it could not keep up
  reconnects @4 :UInt32;
}

# one target's answer before going live
struct CheckResult {
  target @0 :Uuid;
  ok     @1 :Bool;
  note   @2 :Text;              # connection, TLS, key, in words a person understands
}

struct RecordingInfo {
  stream @0 :Uuid;
  name   @1 :Text;
  file   @2 :Text;              # IRI at fs-files, or the place the sink chose
  bytes  @3 :UInt64;
  from   @4 :Timestamp;
  to     @5 :Timestamp;
}

interface StatusListener {
  stream     @0 (stream :Stream) -> stream;
  target     @1 (target :Target) -> stream;
  recording  @2 (recording :RecordingInfo) -> stream;
}

# root capability. Not in this contract yet, each reported unsupported: other ways in (SRT, RTSP, WHIP),
# other ways out (HLS, DASH, WHEP), source.pull, viewer.authorize, hook.send, publishing not live,
# capture; and everything that decodes or encodes.
interface Streaming {
  capabilities  @0 () -> (capabilities :CapabilityList);
  supportReport @1 () -> (report :SupportReport);                 # provider rule (ADR 0045)
  streams       @2 () -> (streams :Streams);
  subscribe     @3 (listener :StatusListener) -> (subscription :Subscription); # stream.subscribe
}

interface Streams {
  # where a sender connects, and the publisher key: shown this once
  create    @0 (name :Text) -> (stream :Stream, ingestUrl :Text, publisherKey :Text);  # stream.create
  # the old key stops at once; a running sender is ended
  rotateKey @1 (id :Uuid) -> (publisherKey :Text);                                     # publisher.authorize
  rename    @2 (id :Uuid, name :Text) -> (stream :Stream);                             # stream.create
  # with its targets; refused while it is live
  delete    @3 (id :Uuid) -> ();                                                       # stream.create
  get       @4 (id :Uuid) -> (found :Bool, stream :Stream);                            # stream.read
  list      @5 (cursor :Cursor) -> (page :StreamPage);                                 # stream.read
  status    @6 (id :Uuid) -> (status :Status);                                         # stream.read
  # ends a running sender
  stop      @7 (id :Uuid) -> (stream :Stream);                                         # stream.stop
  setRecording @8 (id :Uuid, on :Bool) -> (stream :Stream);                            # recording.start

  # A target added or switched on during a live stream starts sending at the next keyframe.
  addTarget  @9 (stream :Uuid, name :Text, url :Text, key :Text) -> (target :Target);  # restream.add
  editTarget @10 (id :Uuid, name :Text, url :Text) -> (target :Target);                # restream.add; off first if it is sending
  # replaces the key; an empty key removes it
  setTargetKey @11 (id :Uuid, key :Text) -> (target :Target);                          # restream.add
  setTargetEnabled @12 (id :Uuid, enabled :Bool) -> (target :Target);                  # restream.add
  removeTarget @13 (id :Uuid) -> ();                                                   # restream.add
  targets    @14 (stream :Uuid) -> (items :List(Target));                              # stream.read
  # tries each enabled target without sending media; takes the state to `checking` and back
  check      @15 (stream :Uuid) -> (results :List(CheckResult));                       # stream.check
}
