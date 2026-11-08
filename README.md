# FreeSynergy Interfaces — the contracts everything agrees on

**This is a building block. It is why you can swap one program for another and your data goes with you.**

FreeSynergy Interfaces holds the contract for each topic: messages, files, people, groups, tasks, events, places, texts and more. A contract says what any provider of that topic can do. Every provider offers all of it, or answers plainly "not possible". Anyone can implement a topic without our programs, and our programs can be replaced by yours.

What is different here: an interface is a target, not something that grows. It is published as a draft, tested by at least two independent users, then fixed as 1.0. After that it changes only by a clear jump to a new version, with old and new offered side by side. Where an open standard exists we follow it; where none exists we write one and offer it to everyone. Each standard has exactly one owning program, so there is one point of truth.

## What it can do today

Latest tag: `v0.47.0`.

- Cap'n Proto schemas for: common types, tasks, messenger, hub, people, groups, files, events, places and texts, and, according to the design file, surveys, history, resources, whiteboard and streaming. The README table lists the first ten; the DESIGN file has the full list.
- The Rust code generated from them is committed, so a normal build needs no schema compiler.
- A shared provider rule: `Unsupported` and `Capabilities` for "what can you do?" and "this is not possible".
- Shared server and client code for local sockets (feature `rpc`): errors in the RFC 9457 form, paging, change feeds.
- Every generated file is checked to be identical to what the official Cap'n Proto compiler produces.
- `fs-mcp`: a first stage that lists, from the contracts, the tools an AI could be offered, with risk classes. It has no server and runs nothing.

## What is not there yet

- No interface is 1.0. All are drafts; most have no second independent user yet.
- Several (files, history, resources, surveys, whiteboard, streaming) are marked provisional because no frontends have been analysed.
- The usage example in the README names an old tag; use the latest.

## Part of FreeSynergy

Interfaces is the common language of the programs. FreeSynergy is a family of programs that are free for everyone, forever, and that keep your data yours. The documentation, the manifesto and the overall plan live in [fs-documentation](https://github.com/FreeSynergy/fs-documentation); the reading version is at <https://freesynergy.github.io/fs-documentation/>. Start with the [manifesto](https://freesynergy.github.io/fs-documentation/manifesto/) if you want to know why we do this.


## For developers

- Workspace crates: `fs-interfaces` (generated code and the provider rule), `regenerate` (rebuilds the generated code with our own compiler [fs-capnpc](https://github.com/FreeSynergy/fs-capnpc)), `fs-mcp`.
- Each schema in `schema/` starts with a header naming status, owner and consumers. Never renumber or remove a field; only append.
- Use it by git tag: `fs-interfaces = { git = "https://github.com/FreeSynergy/fs-interfaces", tag = "v0.44.0" }`.
- After a schema change: `cargo run -p regenerate` and `cargo test -p fs-interfaces`. No C++ toolchain needed.
- Design and the full state table: [DESIGN.md](DESIGN.md).
- Licence: MIT OR Apache-2.0.

## Reference

The public interface contracts of [FreeSynergy](https://freesynergy.github.io/fs-documentation/): Cap'n Proto schemas
(`schema/`) plus the Rust code generated from them (`crates/fs-interfaces`). Anyone can implement a topic without our programs.

Decisions: ADR 0045 (every topic one interface, interfaces are targets), ADR 0050 (one standard, one owning program, one
interface), ADR 0040 (pure-Rust Cap'n Proto compiler, generated code committed). Catalogue:
[interfaces](https://freesynergy.github.io/fs-documentation/en/interfaces/).

### Rules

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

### Interfaces

| Schema | Topic | Owner program | Status |
|--------|-------|---------------|--------|
| `common.capnp` | Shared types: ids, envelope, change feed, provider rule | fs-objects (proposed) | draft |
| `tasks.capnp` | Tasks (JSCalendar `Task`) | fs-tasks | draft |
| `messenger.capnp` | Messages | fs-messenger | draft |
| `hub.capnp` | Hub: register, activity log, sessions | fs-hub | draft |
| `people.capnp` | People, relations, roles, AI marking (JSContact) | fs-people | draft |
| `groups.capnp` | Groups, nesting, membership, invitations, delegation | fs-groups | draft |
| `files.capnp` | Files, folders, resumable upload; backends that keep the bytes | fs-files | draft |
| `events.capnp` | Events (JSCalendar `Event`), instances, invitations | fs-calendar | draft |
| `places.capnp` | Places (GeoJSON); sources of map tiles | fs-maps | draft |
| `texts.capnp` | Texts: text + layers, links | fs-wiki | draft |
| `surveys.capnp` | Surveys, answers, results; AI participation per survey | fs-surveys | draft |
| `history.capnp` | History as an observer of the activity log | fs-history | draft |
| `resources.capnp` | Resources: what we have, need and who has time | fs-resources | draft |
| `whiteboard.capnp` | Boards, objects, strokes | fs-whiteboard | draft |
| `streaming.capnp` | Streams: receive, forward, record | fs-streaming | draft |
| `store.capnp` | The store: sources, available, install, update | fs-store | draft |

Each schema file starts with a header naming status, owner and consumers.

### Use

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

### Regenerate

The generated `crates/fs-interfaces/src/*_capnp.rs` are committed, so a normal build needs no schema compiler. After a
schema change, from the repository root (no C++ toolchain needed):

```text
cargo run -p regenerate
cargo test -p fs-interfaces
```

`regenerate` uses our own compiler [`capnp-compiler`](https://github.com/FreeSynergy/fs-capnpc) with `capnpc` 0.27. The test
`generated_matches_official` checks that the committed files are identical to what the official compiler
(`capnpc-embedded` 0.2, capnp 1.4.0 as WASM) and our compiler produce.
