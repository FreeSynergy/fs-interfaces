# fs-store contract — DRAFT 0.1 (2026-11-08)
#
# STATUS: DRAFT, PROVISIONAL. Owner program: fs-store (FreeSynergy.Store, formerly "launcher"). Lives in
# fs-interfaces (ADR 0045). Consumers so far: the CLI `fs-store` (1 of 2); the Blinc GUI will be the second.
#
# The store installs, updates and removes FreeSynergy programs from Git sources. A source holds an
# AppStream catalog (`catalog.xml`) and its detached OpenPGP signature; every release file is checked
# against the trusted key before anything is installed, a failed update is rolled back, and a running
# program is never replaced. The store updates itself by `update("fs-store")`. Design: fs-store/DESIGN.md.
#
# Names of packages are AppStream component ids. Versions are `major.minor.patch` with an optional
# `-alpha|beta|rc.N`. The built-in reference source can be switched off but never removed.
# Unset = empty text. Facets (ADR 0061) are listed by the program that offers them; installing a facet
# installs its program.
#
# Risk marks (ADR 0021), set by the engine, never by the caller: `source.add`, `source.remove`,
# `source.enable`, `package.install`, `package.update` are writes; `package.update` of a running program
# is refused. Nothing here reads anything sensitive.
#
# Evolution rules (Cap'n Proto): never renumber or remove fields/methods; only append.
# 0.0.19: `Package` got fields @7 to @18, `Release`, `Origin`, `Trust`, `CapabilityRef` and `Picture` are new; nothing was renumbered.

@0xd8a3f61b9c27e504;

using C = import "common.capnp";
using Uuid = C.Uuid;
using CapabilityList = C.CapabilityList;
using SupportReport = C.SupportReport;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;
using Timestamp = C.Timestamp;

enum PackageKind {
  program   @0;
  frontend  @1;
  component @2;
  theme     @3;
}

enum PackageFormat {
  tarGz    @0;   # unpacked below the user's directory, no root needed
  rpm      @1;   # through dnf; the person confirms root themselves
  flatpak  @2;
  appImage @3;
  deb      @4;   # through apt on Debian and Ubuntu; the person confirms root themselves
  exe      @5;   # Windows setup.exe; the store does not install it itself yet (Unsupported)
}

enum JobPhase {
  pending    @0;
  verified   @1;   # signature and checksum are good
  installing @2;
  installed  @3;
  failed     @4;   # then rolledBack, or rollbackFailed
  rolledBack @5;
  rollbackFailed @6;   # the undo failed too: the person must look; `failure` says why
}

# Why a job failed, so a client can say it in its own words. Clients must tolerate values they do not know.
enum JobFailure {
  none               @0;
  fetch              @1;   # a source or a file could not be read
  catalogSignature   @2;   # the catalog is not signed by a trusted key; nothing in it is believed
  releaseSignature   @3;
  checksum           @4;
  notOffered         @5;
  unsupportedFormat  @6;
  downgrade          @7;   # the catalog offers an older version than the installed one
  running            @8;   # the program is running; it is never replaced
  install            @9;   # unpacking or switching failed
}

struct Source {
  id        @0 :Text;
  url       @1 :Text;    # Git address
  enabled   @2 :Bool;
  reference @3 :Bool;    # the built-in FreeSynergy source
}

struct Package {
  name      @0 :Text;    # AppStream component id
  kind      @1 :PackageKind;
  version   @2 :Text;
  format    @3 :PackageFormat;
  source    @4 :Text;    # source id
  summary   @5 :Text;
  facets    @6 :List(Text);
  # Appended in 0.0.19 (ADR 0068; information per package). All optional: a catalog that does not say leaves the default
  # (0, empty), and a client that does not know the fields reads the package as before.
  size      @7 :UInt64;                  # bytes of the release file; 0 = not said
  released  @8 :Timestamp;               # release date of `version`; `unixMillis` 0 = not said
  releases  @9 :List(Release);           # changes per version, newest first; may be empty
  origin    @10 :Origin;
  trust     @11 :Trust;
  # What the package offers, needs and can use (ADR 0068). Names come from the list of `fs-interfaces`
  # (`fs_interfaces::capabilities`, `schema/capabilities.txt`); a name the reader does not know is shown, never dropped.
  provides   @12 :List(CapabilityRef);   # `version` = the level offered
  requires   @13 :List(CapabilityRef);   # `version` = the least level needed (mandatory)
  recommends @14 :List(CapabilityRef);   # `version` = the least level that helps (optional)
  iconSha256 @15 :Text;                  # lower-case hex of the icon file; empty = the catalog has no icon (shown as missing)
  pictures   @16 :List(Picture);         # screenshots; only https, with checksum and licence (else the store lists them as withheld)
  description :union {                   # the long description: text, or a block of the wiki (type only, nothing is loaded here)
    text      @17 :Text;                 # default arm: unset = empty text
    wikiBlock @18 :Text;                 # id of a wiki block (ADR 0031)
  }
}

# One entry of the change list of a package.
struct Release {
  version  @0 :Text;
  released @1 :Timestamp;                # `unixMillis` 0 = not said
  changes  @2 :Text;                     # plain text; empty = not said
}

# Where a package comes from (mirror, ADR 0068). All parts optional.
struct Origin {
  source          @0 :Text;              # https address the maker publishes it at (the mirror's upstream)
  maker           @1 :Text;              # name of the maker (organisation or person)
  makerSignature  @2 :Text;              # the maker's own signature, as the catalog gives it (fingerprint or address); empty = none
  sourceCode      @3 :Text;              # https address of the source code of exactly this version, mirrored with the package
}

# How far a package can be trusted, as the store verified it. Clients must tolerate values they do not know.
enum Trust {
  unknown      @0;                       # not said
  checksumOnly @1;                       # only a checksum
  ourKeyOnly   @2;                       # signed by the key of the source
  makerAndMirror @3;                     # the maker's signature checked, then signed by the mirror
}

# A capability (ADR 0068): a name of the list in `fs-interfaces` and a level `major.minor`; empty level = any.
struct CapabilityRef { name @0 :Text; version @1 :Text; }

struct Picture {
  url     @0 :Text;                      # https only
  sha256  @1 :Text;                      # lower-case hex
  license @2 :Text;                      # SPDX id or text; empty = unclear (the store withholds the picture)
}

# A catalog entry the store refused to offer, with the reason; never dropped silently.
struct Skipped { id @0 :Text; reason @1 :Text; }

struct Job {
  id      @0 :Text;
  package @1 :Text;
  phase   @2 :JobPhase;
  problem @3 :Text;      # human-readable detail; empty otherwise
  version @4 :Text;      # the version being installed
  failure @5 :JobFailure;
  source  @6 :Text;      # source id it came from
  signer  @7 :Text;      # fingerprint that signed it, upper-case hex; empty before it is verified
}

interface Store {
  capabilities  @0 () -> (capabilities :CapabilityList);
  supportReport @1 () -> (report :SupportReport);                 # provider rule (ADR 0045)
  sources       @2 () -> (list :List(Source));                    # source.read
  # Allowed schemes: https, ssh and file (file only for a source the person adds on this machine). A release
  # address in a catalog must be https, or file when the catalog itself came from a file source.
  addSource     @3 (url :Text) -> (source :Source);               # source.add
  setEnabled    @4 (id :Text, enabled :Bool) -> ();               # source.enable
  removeSource  @5 (id :Text) -> ();                              # source.remove; refused for the reference source
  available     @6 () -> (list :List(Package), skipped :List(Skipped));  # package.read
  installed     @7 () -> (list :List(Package));                   # package.read
  # `source` empty = the best verified offer of all enabled sources; with two sources offering one name
  # the caller names the source, or the call is refused as ambiguous
  install       @8 (name :Text, source :Text) -> (job :Job);                  # package.install
  update        @9 (name :Text, source :Text) -> (job :Job);                  # package.update; "fs-store" updates the store itself
  job           @10 (id :Text) -> (found :Bool, job :Job);        # package.read
  # changes of jobs (phase); each job is announced as `created` and then `updated`
  subscribe     @11 (listener :ChangeListener) -> (subscription :Subscription);
  # appended after the review of 2026-11-08
  uninstall     @12 (name :Text) -> (job :Job);                   # package.uninstall; refused while running
  refresh       @13 () -> (skipped :List(Skipped));               # source.refresh: reads every enabled source again
}
