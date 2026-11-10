# fs-surveys contract — DRAFT 0.1 (2026-11-06)
#
# STATUS: DRAFT, PROVISIONAL. Owner program: fs-surveys. Lives in fs-interfaces (ADR 0045).
# Consumers so far: none (0 of 2). No survey frontends are analysed yet; the requirements say this
# contract comes from our architecture and general knowledge and must be checked against them.
#
# A survey is one object: an AS2 `OrderedCollection` of AS2 `Question`s plus our own profile, for what
# AS2 lacks (typed questions, methods, conditions, anonymity; docs/en/programs/apps/surveys/design.md).
# A poll is a survey with one question.
#
# Anonymity is fixed when a survey is published and cannot change after that, widening included, so
# it can never get weaker on the way. Anonymous answers are ballots: they carry no person, no time,
# no id of their own and no order, and they never reach the activity log or `subscribe`.
#
# Anonymous results are released in batches of at least `minGroup` ballots, when the survey closes or
# widens, never per read: comparing two readings then shows a batch, not the newest ballot. A batch
# knows its stage, which is the one coarse time that is kept. Smaller rests stay held back and are
# counted in `heldBack`. Number and date values are shown only as a mean, cross-tabulation and rows
# only for `named`. Unanimous groups stay recognisable; that limit belongs to any poll.
#
# Groups are ids of groups.capnp, people are IRIs of people.capnp, tasks are IRIs of tasks.capnp;
# they are only referred to here (ADR 0050).
#
# Evolution rules (Cap'n Proto): never renumber or remove fields/methods; only append.

@0xd6b1f4a27c3e9085;

using C = import "common.capnp";
using T = import "types.capnp";
using Uuid = C.Uuid;
using Cursor = C.Cursor;
using Envelope = C.Envelope;
using CapabilityList = C.CapabilityList;
using SupportReport = C.SupportReport;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;

enum SurveyState { draft @0; open @1; closed @2; archived @3; }

# the promise made to respondents (design.md, "Anonymity")
enum Anonymity {
  named          @0;    # the organiser sees who answered what
  anonymousOpen  @1;    # no identity stored; one person can answer several times
  anonymousOnce  @2;    # one token per invited person, not traceable by design; the limit is stated
  public         @3;    # anyone, no account, protected against mass answers (`survey.publicLink`)
}

enum QuestionKind {
  singleChoice   @0;    # AS2 `oneOf`
  multipleChoice @1;    # AS2 `anyOf`
  text           @2;
  number         @3;
  date           @4;
  scale          @5;    # one value in [min, max]
  ranking        @6;    # options in order of preference
  optionScores   @7;    # one value in [min, max] for each option; also resistance and objections
}

# how a question is decided; one strategy each, and the same ballots can be evaluated by several
enum Method {
  plurality          @0;
  majority           @1;    # more than half
  supermajority      @2;    # at least `thresholdPercent`
  approval           @3;
  score              @4;
  schulze            @5;
  instantRunoff      @6;
  borda              @7;
  consent            @8;    # passes without an objection (score 1 of 0..1)
  systemicConsensing @9;    # least resistance, 0 to 10
}

struct Choice { id @0 :Text; label @1 :Text; }

enum ConditionOp { answered @0; equals @1; includes @2; atLeast @3; atMost @4; }

# ask this question only if another one was answered this way (FHIR `enableWhen` as the model)
struct Condition {
  question @0 :Uuid;
  op       @1 :ConditionOp;
  value    @2 :Text;          # choice id or number as text; unused for `answered`
}

struct Question {
  id          @0 :Uuid;
  kind        @1 :QuestionKind;
  text        @2 :Text;
  required    @3 :Bool;       # question.validate, with the bounds below
  choices     @4 :List(Choice);
  min         @5 :Int32;      # scale and optionScores
  max         @6 :Int32;
  method      @7 :Method;     # the one shown first; meaningful with choices
  thresholdPercent @8 :UInt8; # supermajority
  conditions  @9 :List(Condition);  # all must hold
  minNumber   @10 :Float64 = nan;   # number; NaN = no bound
  maxNumber   @11 :Float64 = nan;
}

struct Survey {
  envelope  @0 :Envelope;     # AS2 `OrderedCollection` envelope; `sensitive` holds the DPV categories
  as2       @1 :Text;         # the survey as AS2 JSON
  state     @2 :SurveyState;
  anonymity @3 :Anonymity;
  questions @4 :List(Question);
  scope     @5 :List(Uuid);   # groups asked, in the order they were added; empty = not bound to a group
  stage     @6 :UInt16;       # 0 at publishing; +1 for each widening
  minGroup  @7 :UInt16 = 5;   # smallest batch of anonymous results; at least 3 on anonymous surveys
  # AI participants (2026-11-07): see the block at the end of the file
  aiAllowed @8 :Bool = false; # an AI may answer only if the creator allowed it for this survey
  creator   @9 :Text;         # person IRI who made it (`createBy`); empty = nobody may allow AIs
}

struct SurveyPage { items @0 :List(Survey); next @1 :Cursor; }

struct Answer {
  question @0 :Uuid;
  union {
    choice   @1 :Text;              # singleChoice
    choices  @2 :List(Text);        # multipleChoice
    text     @3 :Text;
    number   @4 :Float64;
    date     @5 :Text;              # RFC 3339 full-date
    scale    @6 :Int32;
    ranking  @7 :List(Text);        # most preferred first
    scores   @8 :List(OptionScore);
  }
}

struct OptionScore { choice @0 :Text; score @1 :Int32; }

# only `named`: an answer that knows its author. Anonymous answers have no such form anywhere.
struct NamedResponse {
  envelope  @0 :Envelope;
  person    @1 :Text;
  answers   @2 :List(Answer);
  submitted @3 :Bool;
  ai        @4 :Bool;         # answered by a participant counted as an AI; kept and shown apart
}

# Tokens never appear here. The provider sends each one to its person directly and keeps only a hash
# that is not linked to the person, so the organiser cannot answer as someone else.
struct InviteReport {
  invited @0 :List(Text);         # people newly invited
  already @1 :List(Text);         # invited before; no new token
  refused @2 :List(Text);         # AI, or not known as a human, and the survey does not allow AIs
}

# who is answering; which arm is needed follows from the anonymity of the survey
struct Credential {
  union {
    none   @0 :Void;              # anonymousOpen
    person @1 :Text;              # named
    token  @2 :Text;              # anonymousOnce
    proof  @3 :Data;              # public: the answer to `Respond.challenge`
    open   @4 :OpenClaim;         # anonymousOpen: anyone, no proof (2026-11-08, owner); `none` is refused there
  }
}

# What an open respondent hands in. `declaredAi` is the answer to "Are you an AI?": a claim, never a
# proof, and required (`unset` is refused). `elapsedMs` and `honeypot` only feed the pattern counts
# (`Summary.patterns`); neither is stored with the ballot.
enum Claim { unset @0; human @1; ai @2; }
struct OpenClaim {
  declaredAi @0 :Claim;
  elapsedMs  @1 :UInt32;            # 0 = not measured
  honeypot   @2 :Text;              # the hidden field; any text in it is a pattern
}

# Patterns of open answers, counted per lane and released with the batch, never per ballot. They are
# shown beside the figures and never used to leave an answer out.
struct Patterns {
  looked     @0 :UInt32;            # open claims that were looked at
  tooFast    @1 :UInt32;
  honeypot   @2 :UInt32;
  straightLine @3 :UInt32;          # the same choice in every row
  burst      @4 :UInt32;
  noTiming   @5 :UInt32;
  sameText   @6 :UInt32;            # ballots whose free text another ballot also holds
}

struct ChoiceCount { choice @0 :Text; count @1 :UInt32; }

struct QuestionSummary {
  question @0 :Uuid;
  answers  @1 :UInt32;
  hidden   @2 :Bool;                # too few answers to show (`minGroup`); everything else is empty
  counts   @3 :List(ChoiceCount);   # choices; scale and number values as text
  mean     @4 :Float64 = nan;       # number, scale; NaN when there is no value
  texts    @5 :List(Text);          # free text, shuffled
}

struct Summary {
  ballots   @0 :UInt32;             # released ones
  hidden    @1 :Bool;
  questions @2 :List(QuestionSummary);
  heldBack  @3 :UInt32;             # answers not released yet (see the header); anonymous: 0 while open
  ai        @4 :Summary;            # the AI part, same shape; counted apart, never in the fields above
  hasAi     @5 :Bool;               # false: no AI part (not allowed and no AI answers); `ai` is then empty
  hasPatterns @6 :Bool;             # anonymousOpen only
  patterns  @7 :Patterns;           # beside the figures of this part; nothing is left out because of them
  voluntary @8 :List(ChoiceCount);  # `key=value` and how often; only after the close and from `minGroup` sets
}

struct OptionResult {
  choice @0 :Text;
  value  @1 :Float64;               # votes, points, resistance: what the method counts
  rank   @2 :UInt16;                # 1 = first; equal values share a rank
}

struct Evaluation {
  method   @0 :Method;
  winners  @1 :List(Text);          # empty = not decided (no majority, an objection, a tie in a round, still open)
  options  @2 :List(OptionResult);
  note     @3 :Text;                # why not decided
  hidden   @4 :Bool;
  heldBack @5 :UInt32;              # answers not counted because they are not released yet
  ai       @6 :Evaluation;          # the same question evaluated over AI answers only
  hasAi    @7 :Bool;
}

struct Decision {
  envelope @0 :Envelope;            # store type `Decision`; AS2 has none
  as2      @1 :Text;
  survey   @2 :Uuid;
  question @3 :Uuid;
  method   @4 :Method;
  outcome  @5 :List(Text);          # choice ids
  note     @6 :Text;
  tasks    @7 :List(Text);          # task IRIs (tasks.capnp)
  thresholdPercent @8 :UInt8;
  stage    @9 :UInt16 = 0xffff;     # the ballots it rests on; 0xFFFF = all stages
}

struct DecisionPage { items @0 :List(Decision); next @1 :Cursor; }

# root capability; ids in the methods below and in `survey.classify`, `poll.embed`, `survey.subscribe`
interface Surveys {
  capabilities  @0 () -> (capabilities :CapabilityList);
  supportReport @1 () -> (report :SupportReport);          # provider rule (ADR 0045)
  designer      @2 () -> (designer :Designer);
  respond       @3 () -> (respond :Respond);
  results       @4 () -> (results :Results);
  decisions     @5 () -> (decisions :Decisions);
  # state changes only, and new answers of `named` surveys; ballots never (survey.subscribe)
  subscribe     @6 (listener :ChangeListener) -> (subscription :Subscription);
  # The data types this program owns and what can be done with them (ADR 0066), for a generated frontend.
  types         @7 () -> (catalog :T.TypeCatalog);
}

interface Designer {
  create       @0 (name :Text, anonymity :Anonymity, scope :List(Uuid)) -> (survey :Survey); # survey.create
  edit         @1 (id :Uuid, name :Text, anonymity :Anonymity, minGroup :UInt16) -> (survey :Survey);  # survey.edit, survey.anonymity; draft only
  classify     @2 (id :Uuid, categories :List(Text)) -> (survey :Survey);                # survey.classify
  addQuestion  @3 (id :Uuid, question :Question) -> (survey :Survey, question :Uuid);    # question.add; the given id is ignored
  editQuestion @4 (id :Uuid, question :Question) -> (survey :Survey);                    # question.edit
  moveQuestion @5 (id :Uuid, question :Uuid, position :UInt16) -> (survey :Survey);      # question.move
  removeQuestion @6 (id :Uuid, question :Uuid) -> (survey :Survey);                      # question.edit
  setConditions @7 (id :Uuid, question :Uuid, conditions :List(Condition)) -> (survey :Survey); # logic.set
  publish      @8 (id :Uuid) -> (survey :Survey);                                        # survey.publish
  close        @9 (id :Uuid) -> (survey :Survey);                                        # survey.close
  reopen       @10 (id :Uuid) -> (survey :Survey);                                       # survey.close
  archive      @11 (id :Uuid) -> (survey :Survey);                                       # survey.close
  delete       @12 (id :Uuid) -> ();                                                     # survey.create
  get          @13 (id :Uuid) -> (found :Bool, survey :Survey);
  list         @14 (cursor :Cursor) -> (page :SurveyPage);
  invite       @15 (id :Uuid, people :List(Text), groups :List(Uuid)) -> (report :InviteReport); # survey.invite, survey.tokens
  # the ids of the people in the new groups come from fs-groups; results stay, anonymity stays
  widen        @16 (id :Uuid, parent :Uuid) -> (survey :Survey);                         # survey.widen
  # like `create`, and records `creator`; only this person can later allow AIs
  createBy     @17 (name :Text, anonymity :Anonymity, scope :List(Uuid), creator :Text) -> (survey :Survey); # survey.create
  # only the creator, only in `draft` or `open`; allowing needs a creator who is a human by the marking
  setAiAllowed @18 (id :Uuid, by :Text, allowed :Bool) -> (survey :Survey);              # survey.aiAllowed
}

interface Respond {
  open   @0 (id :Uuid) -> (found :Bool, survey :Survey);                                 # survey.open
  # `named` only; anonymous drafts stay on the respondent's device
  save   @1 (id :Uuid, person :Text, answers :List(Answer)) -> (response :NamedResponse); # response.save
  resume @2 (id :Uuid, person :Text) -> (found :Bool, response :NamedResponse);           # response.save
  submit @3 (id :Uuid, credential :Credential, answers :List(Answer)) -> ();             # response.submit
  # work to do before a `public` answer, bound to this survey; harder under load
  challenge @4 (id :Uuid) -> (challenge :Data, difficulty :UInt8);                       # survey.publicLink
  # details given after the answers (age group, role); tied to no ballot; limits in the program's DESIGN.md
  voluntary @5 (id :Uuid, details :List(Voluntary)) -> ();                               # response.voluntary
}

struct Voluntary { key @0 :Text; value @1 :Text; }

interface Results {
  # stage 0xFFFF = all stages, otherwise that stage only
  summary  @0 (id :Uuid, stage :UInt16 = 0xffff) -> (summary :Summary);                   # result.summary
  evaluate @1 (id :Uuid, question :Uuid, method :Method, thresholdPercent :UInt8, stage :UInt16 = 0xffff) -> (evaluation :Evaluation); # survey.method
  # `named`: one row per response; anonymous: totals only. Media type text/csv; others are Unsupported
  export   @2 (id :Uuid, mediaType :Text) -> (data :Data);                                # result.export
  # `named` only
  filter   @3 (id :Uuid, question :Uuid, choice :Text, stage :UInt16 = 0xffff) -> (summary :Summary); # result.filter
  # the AI answers as their own CSV (first column `participants` = `ai`); `export` never holds them
  exportAi @4 (id :Uuid, mediaType :Text) -> (data :Data);                                # result.export
}

interface Decisions {
  record   @0 (survey :Uuid, question :Uuid, method :Method, thresholdPercent :UInt8, stage :UInt16 = 0xffff, outcome :List(Text), note :Text) -> (decision :Decision); # decision.record
  get      @1 (id :Uuid) -> (found :Bool, decision :Decision);                            # decision.read
  list     @2 (survey :Uuid, cursor :Cursor) -> (page :DecisionPage);                     # decision.read
  linkTask @3 (id :Uuid, task :Text) -> (decision :Decision);                             # decision.link
  unlinkTask @4 (id :Uuid, task :Text) -> (decision :Decision);                           # decision.link
  ofTask   @5 (task :Text) -> (decisions :List(Decision));                                # decision.read
}

# AI participants (2026-11-07, owner). An AI takes part only if the creator allowed it for this survey
# (`aiAllowed`, default false). Whether a participant is an AI is told by the marking the host injects
# (as in fs-groups `AiMarking`), never by what the participant says. Without an answer from the marking,
# or without an identity, a participant is treated as an AI. AI answers are always counted and shown
# apart: the `ai` parts of `Summary` and `Evaluation`, `exportAi`, `NamedResponse.ai`; the fields outside
# `ai` are people only. Anonymous: the class is fixed when a token is issued, and its ballots are
# released in their own batches. A person can hand a token to an AI; that limit is stated, not solved.
# `anonymousOpen` takes `Credential.person` only to ask the marking; it is not stored.
