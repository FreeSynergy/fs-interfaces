# fs-resources contract — DRAFT 0.1 (2026-11-06)
#
# STATUS: DRAFT, PROVISIONAL. Owner program: fs-resources. Lives in fs-interfaces (ADR 0045).
# Consumers so far: none (0 of 2). No standard for a resource model was found and no frontends of this
# category are analysed, so the model is our own (docs/en/programs/apps/resources/index.md).
#
# What we have (things, rooms, material), who has time (capacity), what we need and who offers it.
# Things and people's time share planning, so they are one program; costs are not part of it yet.
#
# A person's capacity and allocations belong to that person. Only they set their capacity and answer
# an allocation, and nobody else reads them. A group sees rounded totals only, from the people who
# agreed to share with that group, and nothing below a fixed number of them. None of it is delivered
# through `subscribe` or to the hub. These methods act for the calling person, which needs a transport
# that knows the caller: until one exists, a server must refuse `Capacities` altogether.
#
# People and groups are IRIs and ids of people.capnp and groups.capnp, places of places.capnp, tasks
# and events of tasks.capnp and events.capnp; they are only referred to here (ADR 0050).
# Half-open time ranges throughout. Unset = null pointer, or empty text. A quantity or a duration of
# zero and a range with `to` <= `from` are refused.
#
# Evolution rules (Cap'n Proto): never renumber or remove fields/methods; only append.

@0x9a4e27c1f6d3b850;

using C = import "common.capnp";
using Uuid = C.Uuid;
using Timestamp = C.Timestamp;
using Cursor = C.Cursor;
using Envelope = C.Envelope;
using CapabilityList = C.CapabilityList;
using SupportReport = C.SupportReport;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;

enum ResourceKind {
  thing    @0;   # lent and given back: a projector, a tent
  room     @1;   # a thing in one place; usually a quantity of 1
  material @2;   # used up: taking it lowers the stock for good
}

struct Resource {
  envelope @0 :Envelope;     # store type `Resource`; AS2 has none
  as2      @1 :Text;
  kind     @2 :ResourceKind;
  quantity @3 :UInt32;       # how many exist now
  unit     @4 :Text;         # free text: piece, litre, ...
  owner    @5 :Text;         # person or group IRI
  place    @6 :Text;         # place IRI; empty = none
  labels   @7 :List(Text);
}

struct ResourcePage { items @0 :List(Resource); next @1 :Cursor; }

struct ResourceFilter {
  kindSet @0 :Bool;          # whether `kind` applies
  kind    @1 :ResourceKind;
  owner   @2 :Text;
  place   @3 :Text;
  label   @4 :Text;
  name    @5 :Text;          # part of the name
}

enum ReservationState {
  requested @0;   # holds nothing yet
  confirmed @1;   # holds its quantity for its time; `material` from `from` until it is finished
  returned  @2;   # ended; for `material`, the stock is now lower by `done`
  cancelled @3;
  declined  @4;
}

struct Reservation {
  envelope @0 :Envelope;     # store type `Reservation`
  resource @1 :Uuid;
  quantity @2 :UInt32;
  from     @3 :Timestamp;
  to       @4 :Timestamp;
  holder   @5 :Text;         # person or group IRI
  state    @6 :ReservationState;
  subject  @7 :Text;         # IRI of the task or event it is for; empty = none
  note     @8 :Text;
  done     @9 :UInt32;       # thing, room: how many came back so far; material: how many were used
}

struct ReservationPage { items @0 :List(Reservation); next @1 :Cursor; }

# one stretch of time in which the same quantity is free
struct FreePeriod { from @0 :Timestamp; to @1 :Timestamp; free @2 :UInt32; }

# ---- people's time ----

# someone's own statement: how many hours a week, in this range; focus hours rather than a flat day
struct Capacity {
  id          @0 :Uuid;
  person      @1 :Text;       # set by the provider: always the calling person
  from        @2 :Timestamp;
  to          @3 :Timestamp;
  minutesPerWeek @4 :UInt32;
  note        @5 :Text;       # seen only by that person
}

enum AllocationState { proposed @0; accepted @1; declined @2; withdrawn @3; }

# a request for someone's hours; counts only once they accept
struct Allocation {
  id       @0 :Uuid;
  person   @1 :Text;
  subject  @2 :Text;          # task IRI
  from     @3 :Timestamp;
  to       @4 :Timestamp;
  minutes  @5 :UInt32;        # in total, over the range
  state    @6 :AllocationState;
  proposer @7 :Text;
}

struct Load { capacityMinutes @0 :UInt64; allocatedMinutes @1 :UInt64; }

struct GroupLoad {
  hidden  @0 :Bool;           # too few people share; everything else stays empty
  people  @1 :UInt32;         # how many share; 0 when hidden
  load    @2 :Load;           # rounded
}

# ---- needs and offers ----

enum NeedState { open @0; covered @1; closed @2; }

struct Need {
  envelope @0 :Envelope;      # store type `Need`
  what     @1 :Text;
  quantity @2 :UInt32;
  unit     @3 :Text;
  from     @4 :Timestamp;     # unset = no time
  to       @5 :Timestamp;
  subject  @6 :Text;          # project, group or task IRI
  state    @7 :NeedState;
  covered  @8 :UInt32;        # sum of accepted offers
}

struct NeedPage { items @0 :List(Need); next @1 :Cursor; }

enum OfferState { offered @0; accepted @1; declined @2; withdrawn @3; }

struct Offer {
  envelope @0 :Envelope;      # store type `Offer`
  need     @1 :Uuid;
  by       @2 :Text;          # person or group IRI
  quantity @3 :UInt32;
  resource @4 :Uuid;          # unset = not one of ours
  state    @5 :OfferState;
  note     @6 :Text;
  reservation @7 :Uuid;       # made on accepting, if the offer names a resource and the need has a time
}

# root capability
interface Resources {
  capabilities  @0 () -> (capabilities :CapabilityList);
  supportReport @1 () -> (report :SupportReport);                 # provider rule (ADR 0045)
  items         @2 () -> (items :Items);
  reservations  @3 () -> (reservations :Reservations);
  capacity      @4 () -> (capacity :Capacities);
  needs         @5 () -> (needs :Needs);
  # resources, reservations, needs and offers; never capacity or allocations (resource.subscribe)
  subscribe     @6 (listener :ChangeListener) -> (subscription :Subscription);
}

interface Items {
  create @0 (kind :ResourceKind, name :Text, quantity :UInt32, unit :Text, owner :Text, place :Text) -> (resource :Resource); # resource.create
  edit   @1 (id :Uuid, name :Text, unit :Text, owner :Text, place :Text, labels :List(Text)) -> (resource :Resource);   # resource.edit
  # refused if `expected` is not the current quantity, or if it would be less than what is held
  setQuantity @2 (id :Uuid, expected :UInt32, quantity :UInt32) -> (resource :Resource);                                                  # resource.edit
  delete @3 (id :Uuid) -> ();                                                                                           # resource.delete
  get    @4 (id :Uuid) -> (found :Bool, resource :Resource);                                                            # resource.read
  list   @5 (filter :ResourceFilter, cursor :Cursor) -> (page :ResourcePage);                                           # resource.read
}

interface Reservations {
  request @0 (resource :Uuid, quantity :UInt32, from :Timestamp, to :Timestamp, holder :Text, subject :Text, note :Text) -> (reservation :Reservation); # reservation.request
  confirm @1 (id :Uuid) -> (reservation :Reservation);            # reservation.decide; refused if not enough is free
  decline @2 (id :Uuid) -> (reservation :Reservation);            # reservation.decide
  cancel  @3 (id :Uuid) -> (reservation :Reservation);            # reservation.request
  # thing, room: `quantity` came back, and it ends when all have. material: `quantity` was used, the
  # rest stays in stock, and it ends now. Refused once it has ended.
  finish  @4 (id :Uuid, quantity :UInt32) -> (reservation :Reservation);  # reservation.decide
  get     @5 (id :Uuid) -> (found :Bool, reservation :Reservation);                       # reservation.read
  ofResource @6 (resource :Uuid, from :Timestamp, to :Timestamp, cursor :Cursor) -> (page :ReservationPage); # reservation.read
  ofHolder   @7 (holder :Text, cursor :Cursor) -> (page :ReservationPage);                # reservation.read
  free    @8 (resource :Uuid, from :Timestamp, to :Timestamp) -> (periods :List(FreePeriod)); # reservation.read
  # one step: a confirmed reservation keeps what it holds, or nothing changes
  change  @9 (id :Uuid, quantity :UInt32, from :Timestamp, to :Timestamp) -> (reservation :Reservation); # reservation.request
}

# Each method acts for the calling person. Another person's record answers exactly like an unknown id.
interface Capacities {
  set      @0 (capacity :Capacity) -> (capacity :Capacity);                 # capacity.set; a range may not overlap another
  remove   @1 (id :Uuid) -> ();                                             # capacity.set
  mine     @2 (from :Timestamp, to :Timestamp) -> (items :List(Capacity));  # capacity.read
  # anyone may ask; the answer is always `proposed` and does not reveal whether the person exists
  propose  @3 (person :Text, subject :Text, from :Timestamp, to :Timestamp, minutes :UInt32) -> (allocation :Allocation); # allocation.propose
  # only that person; can be repeated, so an acceptance can be taken back
  answer   @4 (id :Uuid, accept :Bool) -> (allocation :Allocation);         # allocation.answer
  withdraw @5 (id :Uuid) -> (allocation :Allocation);                       # allocation.propose; only who proposed it
  allocations @6 (from :Timestamp, to :Timestamp) -> (items :List(Allocation)); # capacity.read; those asked of me
  proposed @7 (from :Timestamp, to :Timestamp) -> (items :List(Allocation)); # allocation.propose; those I asked for, with the answer
  load     @8 (from :Timestamp, to :Timestamp) -> (load :Load);             # capacity.read
  # off until the person says so, for each group separately
  shareWithGroup @9 (group :Uuid, share :Bool) -> ();                       # capacity.share
  sharedGroups @10 () -> (groups :List(Uuid));                              # capacity.share
  groupLoad @11 (group :Uuid, from :Timestamp, to :Timestamp) -> (load :GroupLoad); # capacity.group
}

interface Needs {
  create @0 (what :Text, quantity :UInt32, unit :Text, from :Timestamp, to :Timestamp, subject :Text) -> (need :Need); # need.create
  edit   @1 (id :Uuid, what :Text, quantity :UInt32, unit :Text, from :Timestamp, to :Timestamp) -> (need :Need);     # need.create
  close  @2 (id :Uuid) -> (need :Need);                                                                               # need.create
  delete @3 (id :Uuid) -> ();                                                                                         # need.create
  get    @4 (id :Uuid) -> (found :Bool, need :Need);                                                                  # need.read
  list   @5 (subject :Text, openOnly :Bool, cursor :Cursor) -> (page :NeedPage);                                      # need.read
  offer  @6 (need :Uuid, by :Text, quantity :UInt32, resource :Uuid, note :Text) -> (offer :Offer);                   # offer.make
  withdraw @7 (offer :Uuid) -> (offer :Offer);                                                                        # offer.make
  # accepting makes a confirmed reservation if the offer names a resource and the need has a time;
  # refused if that is not free
  decide @8 (offer :Uuid, accept :Bool) -> (offer :Offer);                                                            # offer.decide
  offers @9 (need :Uuid) -> (items :List(Offer));                                                                     # need.read
}
