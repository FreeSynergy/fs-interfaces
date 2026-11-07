# fs-people contract — DRAFT 0.1 (2026-11-06)
#
# STATUS: DRAFT. Owner program: fs-people. Lives in fs-interfaces (ADR 0045).
# Consumers so far: none (0 of 2). Provider so far: fs-people-core (store-backed).
#
# One record per person, several roles, no inheritance, no duplicates (ADR 0052).
#   Relation = what someone is to me: JSContact `relatedTo` / vCard RELATED. Part of the card.
#   Role     = what someone is within a group or organisation: org:Role, org:Membership. NOT part of
#              the card, so an export of a card never reveals a role (ADR 0029).
# fs-people holds the person; the owner of a role holds that role's fields. Role definitions are
# objects loaded as configuration; there is no fixed list of roles in any program.
#
# AIs (ADR 0054): a marked participant = JSContact `kind` "application" (RFC 9553 §2.1.4) plus the
# role "ai", because plain "application" also covers software that is no AI. Nothing restricts them.
# Naming the operator is planned, its form is open; `RoleAssignment.details` leaves room for it.
# Locking AIs out is not part of this draft.
#
# A person travels as JSContact JSON text, which stays authoritative (ADR 0014, 0050), next to the
# AS2 envelope. Evolution rules (Cap'n Proto): never renumber or remove fields/methods; only append.

@0xf41de01e6495cb24;

using C = import "common.capnp";
using Uuid = C.Uuid;
using Cursor = C.Cursor;
using Envelope = C.Envelope;
using SupportReport = C.SupportReport;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;

struct Person {
  envelope  @0 :Envelope;              # AS2 `Profile` envelope (id, iri, name, context "people")
  jscontact @1 :Text;                  # the JSContact `Card` as JSON (authoritative content)
  roles     @2 :List(RoleAssignment);  # only what the caller may see (ADR 0029)
  revision  @3 :UInt64;                # changes at every write of the card (for updateIf); compare for equality only, it is no counter
}

# one page of people in name order
struct PersonPage { items @0 :List(Person); next @1 :Cursor; }

# all members optional; empty = no restriction; combined with AND
struct PersonFilter {
  kind @0 :Text;                       # JSContact kind
  role @1 :Text;                       # role id
  term @2 :Text;                       # substring of name, nickname, e-mail, phone, organisation
}

struct ImportReport {
  created @0 :List(Uuid);
  skipped @1 :List(Text);              # uids that already existed
  failed  @2 :List(Text);
  unclear @3 :List(Text);              # stored, with a sensitive marking that names no category (ADR 0029)
}

# ---- Relations: what someone is to me (part of the card) ----

struct Relation {
  uid      @0 :Text;                   # uid of the related card
  relation @1 :Text;                   # RFC 9553 §2.1.8 type, e.g. "friend", or <domain>:<value>
}

# ---- Same person: two cards stay, a link connects them (link, don't merge) ----

struct SamePersonProposal {
  a       @0 :Uuid;
  b       @1 :Uuid;
  reasons @2 :List(Text);
}

# ---- Roles: objects, ours and yours defined the same way (ADR 0052) ----

struct Localized { language @0 :Text; text @1 :Text; }

struct RoleField {
  name        @0 :Text;
  standard    @1 :Text;                # the standard it follows; empty = none
  description @2 :List(Localized);
}

enum RoleScope { personal @0; group @1; public @2; }

struct RoleDefinition {
  id          @0 :Text;                # full IRI: the file's base plus the short id
  names       @1 :List(Localized);
  description @2 :List(Localized);
  standard    @3 :Text;                # the standard it follows; empty = none (a gap)
  fields      @4 :List(RoleField);     # held by the owner, not by fs-people
  owner       @5 :Text;                # the program responsible for it
  scope       @6 :RoleScope;
  cardKind    @7 :Text;                # cards with this role must have this kind; empty = any
  type        @8 :Text;                # `org:Role` or `skos:Concept`; empty = `skos:Concept` (`org:Role` is a subclass of it, org 5.3.5)
  broader     @9 :List(Text);          # full IRIs of more general roles (skos:broader); may point into another file
  closeMatch  @10 :List(Text);         # full IRIs of the same idea elsewhere (skos:closeMatch); may point into another file
  standardRefs @11 :List(Text);        # addresses of the standard's terms (rdfs:seeAlso)
}

struct RoleAssignment {
  id      @0 :Uuid;
  person  @1 :Uuid;
  role    @2 :Text;                    # role id
  scope   @3 :Text;                    # IRI of the group; empty = personal
  details @4 :Text;                    # JSON object; lossless; for roles owned by fs-people
}

# ---- The interface; capability ids in comments ----

interface People {
  supportReport @0 () -> (report :SupportReport);
  persons       @1 () -> (persons :Persons);
  roles         @2 () -> (roles :Roles);
  subscribe     @3 (listener :ChangeListener) -> (subscription :Subscription);   # person.subscribe
}

interface Persons {
  create         @0  (jscontact :Text) -> (person :Person);                      # person.create
  update         @1  (id :Uuid, jscontact :Text) -> (person :Person);            # person.update
  delete         @2  (id :Uuid) -> ();                                           # person.delete
  get            @3  (id :Uuid) -> (found :Bool, person :Person);                # person.read
  getByUid       @4  (uid :Text) -> (found :Bool, person :Person);               # person.read
  list           @5  (filter :PersonFilter, cursor :Cursor) -> (page :PersonPage); # person.read
  findByDescribes @6 (iri :Text) -> (items :List(Person));                       # person.find
  findByEmail    @7  (email :Text) -> (items :List(Person));                     # person.find
  findByPhone    @8  (phone :Text) -> (items :List(Person));                     # person.find
  importText     @9  (mediaType :Text, text :Text) -> (report :ImportReport);    # person.import
  # A sensitive card, or a sensitive member of a list, goes along only if its uid is in `released`
  # (ADR 0029). A refused card is a problem; members left out of a list are named in `withheld`.
  exportText     @10 (id :Uuid, mediaType :Text, released :List(Text)) -> (text :Text, withheld :List(Text)); # person.export
  relate         @11 (id :Uuid, otherUid :Text, relation :Text) -> (person :Person);   # relation.write
  unrelate       @12 (id :Uuid, otherUid :Text, relation :Text) -> (person :Person);   # relation.write
  relations      @13 (id :Uuid) -> (items :List(Relation));                      # relation.read
  proposals      @14 () -> (items :List(SamePersonProposal));                    # same.propose
  confirmSame    @15 (a :Uuid, b :Uuid) -> (link :Uuid);                         # same.link
  unlinkSame     @16 (link :Uuid) -> ();                                         # same.link
  linkedTo       @17 (id :Uuid) -> (ids :List(Uuid));                            # same.read
  # JSContact group cards as lists of people. Rights and belonging are fs-groups (ADR 0050).
  createList     @18 (name :Text) -> (person :Person);                           # cardgroup.write
  addToList      @19 (list :Uuid, member :Uuid) -> (person :Person);             # cardgroup.write
  removeFromList @20 (list :Uuid, member :Uuid) -> (person :Person);             # cardgroup.write
  listMembers    @21 (list :Uuid) -> (items :List(Person));                      # cardgroup.read
  listsOf        @22 (member :Uuid) -> (items :List(Person));                    # cardgroup.read
  updateIf       @23 (id :Uuid, revision :UInt64, jscontact :Text) -> (person :Person);  # person.update, only if the person still has `revision`; otherwise it fails with a reason starting `conflict:` and writes nothing (compare-and-set; atomic inside one provider process only)
}

interface Roles {
  define     @0 (definition :RoleDefinition) -> ();                              # role.define
  undefine   @1 (id :Text) -> ();                                                # role.define
  loadConfig @2 (json :Text) -> ();                                              # role.configure: checked, then swapped at once
  get        @3 (id :Text) -> (found :Bool, definition :RoleDefinition);         # role.read
  list       @4 () -> (items :List(RoleDefinition));                             # role.read
  assign     @5 (person :Uuid, role :Text, scope :Text, details :Text) -> (assignment :RoleAssignment); # role.assign
  unassign   @6 (assignment :Uuid) -> ();                                        # role.assign
  rolesOf    @7 (person :Uuid) -> (items :List(RoleAssignment));                 # role.read
  peopleWith @8 (role :Text, scope :Text) -> (ids :List(Uuid));                  # role.read
  markAi     @9 (person :Uuid) -> (assignment :RoleAssignment);                  # ai.mark
  isAi       @10 (person :Uuid) -> (ai :Bool);                                   # ai.mark
}
