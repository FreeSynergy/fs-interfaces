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

@0xd8a3f61b9c27e504;

using C = import "common.capnp";
using Uuid = C.Uuid;
using CapabilityList = C.CapabilityList;
using SupportReport = C.SupportReport;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;

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
}

enum JobPhase {
  pending    @0;
  verified   @1;   # signature and checksum are good
  installing @2;
  installed  @3;
  failed     @4;   # always followed by rolledBack
  rolledBack @5;
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
}

# A catalog entry the store refused to offer, with the reason; never dropped silently.
struct Skipped { id @0 :Text; reason @1 :Text; }

struct Job {
  id      @0 :Text;
  package @1 :Text;
  phase   @2 :JobPhase;
  problem @3 :Text;      # why it failed; empty otherwise
}

interface Store {
  capabilities  @0 () -> (capabilities :CapabilityList);
  supportReport @1 () -> (report :SupportReport);                 # provider rule (ADR 0045)
  sources       @2 () -> (list :List(Source));                    # source.read
  addSource     @3 (url :Text) -> (source :Source);               # source.add
  setEnabled    @4 (id :Text, enabled :Bool) -> ();               # source.enable
  removeSource  @5 (id :Text) -> ();                              # source.remove; refused for the reference source
  available     @6 () -> (list :List(Package), skipped :List(Skipped));  # package.read
  installed     @7 () -> (list :List(Package));                   # package.read
  install       @8 (name :Text) -> (job :Job);                    # package.install
  update        @9 (name :Text) -> (job :Job);                    # package.update; "fs-store" updates the store itself
  job           @10 (id :Text) -> (found :Bool, job :Job);        # package.read
  # changes of jobs (phase); each job is announced as `created` and then `updated`
  subscribe     @11 (listener :ChangeListener) -> (subscription :Subscription);
}
