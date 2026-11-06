# fs-interfaces

The public interface contracts of [FreeSynergy](https://freesynergy.github.io/fs-documentation/): Cap'n Proto schemas
(`schema/`) plus the Rust code generated from them (`crates/fs-interfaces`). Anyone can implement a topic without our programs.

Decisions: ADR 0045 (every topic one interface, interfaces are targets), ADR 0050 (one standard, one owning program, one
interface), ADR 0040 (pure-Rust Cap'n Proto compiler, generated code committed). Catalogue:
[interfaces](https://freesynergy.github.io/fs-documentation/en/interfaces/).

## Rules

1. **A standard comes first.** If one exists, the interface follows it. If not, we write one and offer it to others.
   **Each standard has exactly one owning program** (the point of truth); every other program goes through its interface.
   No second model of the same thing.
2. **An interface is a target.** It is published as a *draft*, tested by at least two independent users (for a provider
   interface: two providers), then fixed as *1.0*. After that it changes only by a **jump** to a new version (old and new are
   offered side by side during the transition), never by gradual growth. Cap'n Proto rule meanwhile: never renumber or
   remove, only append.
3. **Like a parent class.** Every provider offers **all** methods and attributes of its interface. If something is missing,
   look for a way to make it work anyway. If there truly is none, the provider answers an explicit **"not possible"**:
   in Cap'n Proto a `Problem` of type `unsupportedProblemType` (`Unsupported`), in Rust a trait method with a default body
   returning `Err(Unsupported)`. Each provider reports what it can do (`SupportReport`, `Capabilities`).
4. **A frontend uses what it wants.** The engine may offer more than a frontend needs.
5. **Plugins can be added and removed while running** (hot plugins).

## Interfaces

| Schema | Topic | Owner program | Consumers | Status |
|--------|-------|---------------|-----------|--------|
| `common.capnp` | Shared types: ids, envelope, change feed, provider rule | `fs-store` (proposed) | all | draft 0.2 |
| `tasks.capnp` | Tasks (JSCalendar `Task`) | `fs-tasks` | `fs-calendar` (1 of 2) | draft 0.1 |
| `messenger.capnp` | Messages | `fs-messenger` | none yet | draft 0.2 |
| `hub.capnp` | Hub: register, activity log, sessions | `fs-hub` | `fs-hub-client` (1 of 2) | draft 0.1 |
| `people.capnp` | People, relations, roles, AI marking (JSContact) | `fs-people` | none yet | draft 0.1 |
| `groups.capnp` | Groups, nesting, membership, invitations, delegation; directories that mirror them | `fs-groups` | none yet | draft 0.1 |
| `files.capnp` | Files, folders, resumable upload; backends that keep the bytes | `fs-files` | none yet | draft 0.8, provisional |
| `events.capnp` | Events (JSCalendar `Event`), instances, invitations | `fs-calendar` | none yet | draft 0.1 |
| `places.capnp` | Places (GeoJSON); sources of map tiles | `fs-maps` | none yet | draft 0.1 |
| `texts.capnp` | Texts: text + layers, links | `fs-wiki` | none yet | draft 0.1 |

Each schema file starts with a header naming status, owner and consumers.

## Use

```toml
fs-interfaces = { git = "https://github.com/FreeSynergy/fs-interfaces", tag = "v0.7.0" }
```

```rust
use fs_interfaces::{common_capnp, tasks_capnp, messenger_capnp, hub_capnp, people_capnp, groups_capnp, files_capnp};
use fs_interfaces::{Capabilities, Unsupported};
```

The provider rule in Rust (see the doc example in `provider.rs`):

```rust
trait Messages {
    fn send(&self, text: &str) -> Result<(), Unsupported>;
    fn edit(&self, _id: &str, _text: &str) -> Result<(), Unsupported> {
        Err(Unsupported::new("message.edit", "this network has no edit"))
    }
    fn capabilities(&self) -> Capabilities;
}
```

## Regenerate

The generated `crates/fs-interfaces/src/*_capnp.rs` are committed, so a normal build needs no schema compiler. After a
schema change, from the repository root (no C++ toolchain needed):

```text
cargo run -p regenerate
cargo test -p fs-interfaces
```

`regenerate` uses our own compiler [`capnp-compiler`](https://github.com/FreeSynergy/fs-capnpc) with `capnpc` 0.27. The test
`generated_matches_official` checks that the committed files are identical to what the official compiler
(`capnpc-embedded` 0.2, capnp 1.4.0 as WASM) and our compiler produce.

## Licence

MIT OR Apache-2.0.
