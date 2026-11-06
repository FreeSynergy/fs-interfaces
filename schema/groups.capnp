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
}

interface Groups {
  supportReport @0 () -> (report :SupportReport);
  items         @1 () -> (items :Items);
  memberships   @2 () -> (memberships :Memberships);
  invitations   @3 () -> (invitations :Invitations);
  delegations   @4 () -> (delegations :Delegations);
  subscribe     @5 (listener :ChangeListener) -> (subscription :Subscription);   # group.subscribe
}

interface Items {
  create    @0 (name :Text, kind :Text) -> (group :Group);                       # group.create
  rename    @1 (id :Uuid, name :Text) -> (group :Group);                         # group.update
  setKind   @2 (id :Uuid, kind :Text) -> (group :Group);                         # group.update
  delete    @3 (id :Uuid) -> ();                                                 # group.delete
  get       @4 (id :Uuid) -> (found :Bool, group :Group);                        # group.read
  list      @5 (cursor :Cursor) -> (page :GroupPage);                            # group.read
  addParent @6 (id :Uuid, parent :Uuid) -> (group :Group);                       # group.nest (a cycle is refused)
  removeParent @7 (id :Uuid, parent :Uuid) -> (group :Group);                    # group.nest
  parents   @8 (id :Uuid) -> (ids :List(Uuid));                                  # group.read
  children  @9 (id :Uuid) -> (ids :List(Uuid));                                  # group.read
  ancestors @10 (id :Uuid) -> (ids :List(Uuid));                                 # group.read
  descendants @11 (id :Uuid) -> (ids :List(Uuid));                               # group.read
}

interface Memberships {
  add       @0 (group :Uuid, member :Member, role :Text) -> (membership :Membership);  # member.add
  remove    @1 (membership :Uuid) -> ();                                         # member.remove
  direct    @2 (group :Uuid) -> (items :List(Membership));                       # member.read
  effective @3 (group :Uuid) -> (items :List(EffectiveMember));                  # member.read: through nested groups
  groupsOf  @4 (person :Text) -> (ids :List(Uuid));                              # member.read: through nested groups
  isMember  @5 (group :Uuid, person :Text) -> (member :Bool);                    # member.read
}

interface Invitations {
  invite  @0 (group :Uuid, invitee :Text, role :Text, expires :Timestamp) -> (invitation :Invitation);  # invite.create
  accept  @1 (id :Uuid, person :Text) -> (membership :Membership);               # invite.answer
  decline @2 (id :Uuid) -> ();                                                   # invite.answer
  revoke  @3 (id :Uuid) -> ();                                                   # invite.create
  list    @4 (group :Uuid) -> (items :List(Invitation));                         # invite.read
}

interface Delegations {
  grant    @0 (group :Uuid, holder :Member, right :Right) -> (delegation :Delegation);  # right.grant
  revoke   @1 (id :Uuid) -> ();                                                  # right.grant
  list     @2 (group :Uuid) -> (items :List(Delegation));                        # right.read
  mayDo    @3 (group :Uuid, person :Text, right :Right) -> (allowed :Bool);      # right.read
}

# ---- Provider interface: a directory that mirrors groups (ADR 0047). Kanidm is one adapter. ----
# fs-groups says what should be; a directory is brought to that state, repeatably, and can heal.

struct DirectoryGroup {
  id      @0 :Uuid;                # the group id of fs-groups
  name    @1 :Text;
  members @2 :List(Text);          # direct people
  nested  @3 :List(Uuid);          # direct groups
}

interface GroupDirectory {
  supportReport @0 () -> (report :SupportReport);
  apply  @1 (group :DirectoryGroup) -> ();                                       # directory.apply: idempotent
  remove @2 (id :Uuid) -> ();                                                    # directory.apply
  read   @3 (id :Uuid) -> (found :Bool, group :DirectoryGroup);                  # directory.read
  list   @4 () -> (items :List(DirectoryGroup));                                 # directory.read
}
