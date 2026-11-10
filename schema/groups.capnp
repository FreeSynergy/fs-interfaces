# fs-groups contract — DRAFT 0.1 (2026-11-06)
#
# STATUS: DRAFT. Owner program: fs-groups. Lives in fs-interfaces (ADR 0045).
# Consumers so far: none (0 of 2). Providers: fs-groups-core; a directory such as Kanidm is mirrored
# through `GroupDirectory` below and is never the point of truth (ADR 0047).
#
# Groups contain groups. Cycles are refused here, since Kanidm would accept them. A group may have
# several parents (a working group in two departments); that answer is a proposal, the owner can
# narrow it to a tree (docs/en/programs/platform/groups.md, question 1).
# Runs without any directory (ADR 0024). A person alone and a group use the same programs.
# Kind, parents, roles, invitations and delegation live here, keyed by the group id (ADR 0047).
# Role ids are the ones of people.capnp; the definitions stay there (ADR 0052).
# Evolution rules (Cap'n Proto): never renumber or remove fields/methods; only append.

@0xb3379cb1bb991be5;

using C = import "common.capnp";
using T = import "types.capnp";
using Uuid = C.Uuid;
using Cursor = C.Cursor;
using Envelope = C.Envelope;
using Timestamp = C.Timestamp;
using SupportReport = C.SupportReport;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;

struct Group {
  envelope @0 :Envelope;           # AS2 `Group` envelope
  as2      @1 :Text;               # the AS2 `Group` as JSON (authoritative content)
  kind     @2 :Text;               # team, project, department, ... free text, not a fixed list
  parents  @3 :List(Uuid);
  rightFlow @4 :RightFlow;         # whether and where this group's rights are passed on; default none
}

struct GroupPage { items @0 :List(Group); next @1 :Cursor; }

# who belongs: a person (IRI of a person in fs-people or an actor) or another group
struct Member {
  union {
    person @0 :Text;
    group  @1 :Uuid;
  }
}

# org:Membership
struct Membership {
  id     @0 :Uuid;
  group  @1 :Uuid;
  member @2 :Member;
  role   @3 :Text;                 # role id of people.capnp; empty = plain member
  # false when the role is a configured admin role and the person is an agent or not known to be a human
  # (ADR 0056): the entry is on record but counts for nothing. A reader that does not know the field sees false.
  inForce @4 :Bool;
}

# which group a person belongs to, and through whom
struct EffectiveMember {
  person @0 :Text;
  via    @1 :List(Uuid);           # path of groups from the asked group to the direct membership
}

enum InvitationState { pending @0; accepted @1; declined @2; revoked @3; expired @4; }

struct Invitation {
  id      @0 :Uuid;
  group   @1 :Uuid;
  invitee @2 :Text;                # IRI or e-mail address
  role    @3 :Text;
  state   @4 :InvitationState;
  expires @5 :Timestamp;
}

# finer than "all or nothing" (Kanidm: all or nothing, kanidm#4641)
enum Right { read @0; addMembers @1; removeMembers @2; manage @3; }

struct Delegation {
  id     @0 :Uuid;
  group  @1 :Uuid;
  holder @2 :Member;
  right  @3 :Right;
  # false for manage held by a person who is an agent or not known to be a human (ADR 0056): on record,
  # but mayDo does not count it. A reader that does not know the field sees false.
  inForce @4 :Bool;
}

interface Groups {
  supportReport @0 () -> (report :SupportReport);
  items         @1 () -> (items :Items);
  memberships   @2 () -> (memberships :Memberships);
  invitations   @3 () -> (invitations :Invitations);
  delegations   @4 () -> (delegations :Delegations);
  subscribe     @5 (listener :ChangeListener) -> (subscription :Subscription);   # group.subscribe
  follows       @6 () -> (follows :Follows);                                     # follow.* (provisional)
  # The data types this program owns and what can be done with them (ADR 0066), for a generated frontend.
  types         @7 () -> (catalog :T.TypeCatalog);
}

interface Items {
  create    @0 (name :Text, kind :Text) -> (group :Group);                       # group.create
  rename    @1 (id :Uuid, name :Text) -> (group :Group);                         # group.update
  setKind   @2 (id :Uuid, kind :Text) -> (group :Group);                         # group.update
  delete    @3 (id :Uuid) -> ();                                                 # group.delete
  get       @4 (id :Uuid) -> (found :Bool, group :Group);                        # group.read
  list      @5 (cursor :Cursor) -> (page :GroupPage);                            # group.read
  addParent @6 (id :Uuid, parent :Uuid) -> (group :Group);                       # group.nest: SUPERSEDED by requestParent @13 (a parent now has to consent); a provider answers unsupported
  removeParent @7 (id :Uuid, parent :Uuid) -> (group :Group);                    # group.nest
  parents   @8 (id :Uuid) -> (ids :List(Uuid));                                  # group.read
  children  @9 (id :Uuid) -> (ids :List(Uuid));                                  # group.read
  ancestors @10 (id :Uuid) -> (ids :List(Uuid));                                 # group.read
  descendants @11 (id :Uuid) -> (ids :List(Uuid));                               # group.read
  setRightFlow @12 (id :Uuid, flow :RightFlow) -> (group :Group);                # group.update
  # Nesting needs the parent's consent (PROVISIONAL: who consents is still the owner's to answer).
  # requestParent only writes a request; the link exists once approveParent ran on behalf of the parent.
  requestParent @13 (child :Uuid, parent :Uuid) -> (request :ParentRequest);     # group.nest (a cycle is refused)
  approveParent @14 (request :Uuid) -> (group :Group);                           # group.nest: returns the child with its new parent
  declineParent @15 (request :Uuid) -> ();                                       # group.nest
  parentRequests @16 (parent :Uuid) -> (items :List(ParentRequest));             # group.nest: any state, oldest first
}

interface Memberships {
  # A group joins a group through Items.requestParent and approveParent (the parent consents), one home for
  # nesting; add refuses a group member.
  # An unknown group id is an error, not an empty answer, in all reads of this interface and Items.
  add       @0 (group :Uuid, member :Member, role :Text) -> (membership :Membership);  # member.add
  remove    @1 (membership :Uuid) -> ();                                         # member.remove
  # direct and effective page like Items.list: empty cursor = start, empty next = the end. A cursor was appended
  # because otherwise a long list cannot be read at all, and the ids for remove only come from direct.
  direct    @2 (group :Uuid, cursor :Cursor) -> (items :List(Membership), next :Cursor);        # member.read
  effective @3 (group :Uuid, cursor :Cursor) -> (items :List(EffectiveMember), next :Cursor);   # member.read: through nested groups
  groupsOf  @4 (person :Text) -> (ids :List(Uuid));                              # member.read: through nested groups
  isMember  @5 (group :Uuid, person :Text) -> (member :Bool);                    # member.read
}

interface Invitations {
  invite  @0 (group :Uuid, invitee :Text, role :Text, expires :Timestamp) -> (invitation :Invitation);  # invite.create
  accept  @1 (id :Uuid, person :Text) -> (membership :Membership);               # invite.answer
  decline @2 (id :Uuid) -> ();                                                   # invite.answer
  revoke  @3 (id :Uuid) -> ();                                                   # invite.create
  list    @4 (group :Uuid) -> (items :List(Invitation));                         # invite.read
  # accept only works for the invitee exactly. For an invitation sent to an e-mail address the caller
  # says which verified address the person proved (PROVISIONAL; nothing is verified by the provider).
  acceptProved @5 (id :Uuid, person :Text, provedAddress :Text) -> (membership :Membership);  # invite.answer
}

interface Delegations {
  grant    @0 (group :Uuid, holder :Member, right :Right) -> (delegation :Delegation);  # right.grant
  revoke   @1 (id :Uuid) -> ();                                                  # right.grant
  list     @2 (group :Uuid) -> (items :List(Delegation));                        # right.read
  mayDo    @3 (group :Uuid, person :Text, right :Right) -> (allowed :Bool);      # right.read
}

# ---- Provider interface: a directory that mirrors groups (ADR 0047). Kanidm is one adapter. ----
# fs-groups says what should be; a directory is brought to that state, repeatably, and can heal.

# Roles are not mirrored yet: a person appears once per group.
struct DirectoryGroup {
  id      @0 :Uuid;                # the group id of fs-groups
  name    @1 :Text;
  members @2 :List(Text);          # people listed in the group: direct, and for a child that is not nested the people of its subtree
  nested  @3 :List(Uuid);          # child groups nested in it (only where both pass up and nothing forbids it)
}

interface GroupDirectory {
  supportReport @0 () -> (report :SupportReport);
  apply  @1 (group :DirectoryGroup) -> ();                                       # directory.apply: idempotent
  remove @2 (id :Uuid) -> ();                                                    # directory.apply
  read   @3 (id :Uuid) -> (found :Bool, group :DirectoryGroup);                  # directory.read
  list   @4 () -> (items :List(DirectoryGroup));                                 # directory.read
  # One value per directory, stable, different for two directories of one provider (for example the
  # URL of the instance). The mirror ledger is kept under it.
  directoryId @5 () -> (id :Text);
  # apply: a person the directory cannot map must not stop the rest. The adapter applies everything else
  # and answers with a problem that names the people left out (Unmapped).
}

# Each group sets itself whether and in which direction its rights are passed on (ADR 0055 point 6).
# The exact meaning of down and up is provisional and lives in the implementing program.
enum RightFlow { none @0; down @1; up @2; }

# A group asks to become a child of another. The link exists only once the parent's side approves.
enum RequestState { pending @0; approved @1; declined @2; }

struct ParentRequest {
  id        @0 :Uuid;
  child     @1 :Uuid;
  parent    @2 :Uuid;
  requester @3 :Text;              # who asked (IRI of the acting person)
  state     @4 :RequestState;
}

# NOTE (provisional): the report of a synchronisation (what was created, left out, unreadable, looped) is
# not part of this contract; no method returns it yet. When a server exposes it, it appends it here.

# ---- Following between independent groups (ADR 0056 "Einseitig folgen"; PROVISIONAL, the form is a proposal) ----
# Not a tree and not nesting: no parents, no cycle rule. Who allows access DELIVERS (a repository turned
# round); the receiver gets an offer (proposal: AS2 `Invite`), its server enters the source WITHOUT any
# confirmation, and the receiver can remove it at any time. Mutual is allowed. Only the giver's own
# rights go, never more than the giver holds (mayDo). The receiver may pass a delivery on only if the
# source set `passOn`; the giver can revoke at any time. Nothing here authenticates `sender`: the
# transport supplies it and the provider only checks that the offer's source lies at that origin.
enum DeliveryState { active @0; revoked @1; }

struct Delivery {
  id        @0 :Uuid;
  group     @1 :Uuid;              # the giving group on this server
  recipient @2 :Text;              # IRI of the receiving group (https://<instance>/groups/<id>)
  rights    @3 :List(Right);       # never manage (provisional)
  passOn    @4 :Bool;              # the receiver may pass it on (not again from the relayed copy)
  via       @5 :List(Uuid);        # empty: own right; one entry: relayed from this source on this server
  state     @6 :DeliveryState;
  origin    @7 :Text;              # IRI of the group whose right this is: the giver itself, or for a relayed copy the first source
}

# What reaches the receiver (the AS2 `Invite` content, proposal): enough to enter the source.
struct Offer {
  delivery  @0 :Uuid;              # id of the delivery at the giver
  source    @1 :Text;              # IRI of the giving group
  recipient @2 :Text;              # IRI of the receiving group
  rights    @3 :List(Right);
  passOn    @4 :Bool;
  as2       @5 :Text;              # the `Invite` as JSON
  origin    @6 :Text;              # IRI of the group whose right this is; equals `source` unless the giver relays
}

enum SourceState { active @0; withdrawn @1; }

# A source the receiving group has entered (the receiver's list of repositories, turned round).
struct Source {
  id     @0 :Uuid;
  group  @1 :Uuid;                 # the receiving group on this server
  offer  @2 :Offer;
  sender @3 :Text;                 # origin the transport named when the offer arrived
  state  @4 :SourceState;
}

interface Follows {
  # giver side
  deliver    @0 (group :Uuid, recipient :Text, rights :List(Right), passOn :Bool) -> (delivery :Delivery, offer :Offer);  # follow.deliver
  passOn     @1 (source :Uuid, recipient :Text, rights :List(Right)) -> (delivery :Delivery, offer :Offer);              # follow.deliver
  revoke     @2 (delivery :Uuid) -> ();                                         # follow.deliver
  deliveries @3 (group :Uuid) -> (items :List(Delivery));                       # follow.read
  # receiver side
  receive    @4 (sender :Text, offer :Offer) -> (source :Source);               # follow.receive: no confirmation
  withdraw   @5 (sender :Text, source :Text, delivery :Uuid) -> (source :Source);  # follow.receive: the giver revoked
  sources    @6 (group :Uuid) -> (items :List(Source));                         # follow.read
  removeSource @7 (id :Uuid) -> ();                                             # follow.receive: the receiver ends it
}
