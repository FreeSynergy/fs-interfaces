# fs-maps contract: places — DRAFT 0.1 (2026-11-06)
#
# STATUS: DRAFT. Owner program: fs-maps. Lives in fs-interfaces (ADR 0045, 0050).
# Consumers so far: none (0 of 2). Provider: fs-maps-core.
# A place travels as GeoJSON Feature text (RFC 7946), which stays authoritative. Where map pictures
# come from is the provider interface `TileSource` below. Evolution rules: only append.

@0xd6ef645f4581a37d;

using C = import "common.capnp";
using Uuid = C.Uuid;
using Timestamp = C.Timestamp;
using Cursor = C.Cursor;
using Envelope = C.Envelope;
using SupportReport = C.SupportReport;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;

struct Place {
  envelope @0 :Envelope;           # AS2 `Place` envelope
  geojson  @1 :Text;               # GeoJSON Feature (authoritative content)
}

struct PlacePage { items @0 :List(Place); next @1 :Cursor; }

struct BBox { west @0 :Float64; south @1 :Float64; east @2 :Float64; north @3 :Float64; }

interface Places {
  supportReport @0 () -> (report :SupportReport);
  create @1 (geojson :Text) -> (place :Place);                                   # place.create
  update @2 (id :Uuid, geojson :Text) -> (place :Place);                         # place.update
  delete @3 (id :Uuid) -> ();                                                    # place.delete
  get    @4 (id :Uuid) -> (found :Bool, place :Place);                           # place.read
  list   @5 (cursor :Cursor) -> (page :PlacePage);                               # place.read
  within @6 (box :BBox, cursor :Cursor) -> (page :PlacePage);                    # place.find
  findByName @7 (term :Text, cursor :Cursor) -> (page :PlacePage);               # place.find
  subscribe  @8 (listener :ChangeListener) -> (subscription :Subscription);      # place.subscribe
  tiles      @9 () -> (source :TileSource);                                      # tile.read: a program has one socket, this is the way to its tiles
}

# ---- Provider interface: where map tiles come from. PMTiles over HTTP, a file, a region package. ----

interface TileSource {
  supportReport @0 () -> (report :SupportReport);
  id   @1 () -> (id :Text);
  tile @2 (z :UInt8, x :UInt32, y :UInt32) -> (found :Bool, mvt :Data);          # tile.read: uncompressed MVT
}
