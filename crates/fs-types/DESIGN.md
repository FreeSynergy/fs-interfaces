# fs-types - Design

The basic types of ADR 0066 and the type description every program reports. Wire form: `schema/types.capnp` (this repo).

## Where it lives, and why (`coding.md` §4, §8)

A workspace crate of `fs-interfaces`, next to `fs-mcp`, not a repository of its own. Reasons: (1) `coding.md` §8 lets repos depend only by tag, and the rules here belong to the contract in `types.capnp`; one tag moves both, so schema and rules cannot drift apart. (2) `fs-interfaces` already holds `values::{EmailAddress, PhoneNumber}`, the first two basic types; they stay where they are (programs use them since before this crate) and `fs-types` adds them to its registry by an Adapter instead of copying them (`coding.md` §4). (3) A new repository means a new GitHub repository, which is outward-facing and not part of this round; it can be split off later by moving the crate (consumers name the crate, not the path). A consumer writes `fs-types = { git = "https://github.com/FreeSynergy/fs-interfaces", tag = "v0.0.N" }`. `fs-interfaces` itself does not depend on `fs-types`.

## Pattern map

| Part | Pattern | Why |
|---|---|---|
| `BasicValue` and one file per kind (`text`, `contact`, `language`, `iri`, `datetime`, `place`, `key`, `keywords`, `name`, `address`, `image`, `choice`) | Value object + Template method (`parse`/`canonical` per type, `check`/`suggest` provided) | valid on construction, equal values have equal canonical text; the trait is the interface (ADR 0043) |
| `language`, `iri`, `datetime`, `place`, `contact` | Adapter over a free crate (`oxilangtag`, `iri-string`, `chrono`, `geo-uri`, `fs-interfaces::values`) | do not rebuild what exists (ADR 0015, `coding.md` §4) |
| `rule.rs` `Rule`, `BasicRules` | Registry of Strategy objects | a generated frontend knows only a field's `BasicType`; it reaches check/normalise/suggest by kind |
| `describe/` `FieldDescription`, `CommandDescription`, `TypeDescription` | Value object + Builder | read from a program, written by chaining |
| `describe/describes.rs` `DescribesType` | Registry entry point (trait first) | a program lists each data type it owns |
| `describe/registry.rs` | Registry | name to description, validated on the way in |
| `describe/validate.rs`, `check.rs` | Template method | one flow for all programs: the admin frontend and a program's own `check` command say the same |
| `describe/wire.rs`, `describe/server.rs` (feature `rpc`) | Adapter | generated capnp types to plain structs, and `TypeCatalog.describe` over a registry |

## Decisions (provisional, marked for the strategy)

* Kinds in this version: the thirteen of the task list plus `text` and `multilineText`. `onlineService`, `timeZone`, `duration`, `recurrence`, `quantity`, `money`, `colour`, `sound`, `video`, formatted text are in `map.md` and are appended to `BasicType` later (append only; a reader meeting a newer kind treats the field as plain text).
* Canonical forms: e-mail as `fs-interfaces` makes it; phone `+<digits>`; language RFC 5646 case; link with a host (`//`) normalised by RFC 3986 syntax rules (scheme and host lower case, dot segments removed, percent codes of unreserved characters decoded and hex digits upper case), link without a host (`mailto:`, `urn:`, `tel:`) only the scheme in lower case; reference kept as written (an identity); date `YYYY-MM-DD`; date-time with offset, `Z` for UTC (two date-times are equal only if instant and offset are; `-00:00`, "offset unknown", is refused); place as `geo:` URI; fingerprint bare upper-case hex; keywords lower case, sorted, `, `-joined.
* Limits (ours, provisional): text 4096 chars, multi-line 65536, URI 2048 bytes, keyword 64 chars and 256 per set, name/address parts 1024 chars.
* Unsafe schemes (provisional): a `Link` may not use `javascript:`, `vbscript:` or `data:`, an `Image` not `javascript:` or `vbscript:` (a frontend may make them clickable); `TypeProblem::UnsafeScheme`. Characters refused in every text: control characters, U+2028/U+2029 and the bidirectional overrides and isolates; multi-line texts and `Address.full` keep `\n`.
* `Reference` has no scheme block (an id is an identity, not a target to open); a frontend must not make references clickable by scheme alone, it shows the target's compact component. A reference target in a description is a bare type name (UpperCamel), no program prefix.
* A description read from the wire is validated leniently: a kind or command kind this version reads as `unknown` passes (with its list or target), so one newer program cannot make the whole catalog unreadable; `register`/`add` stay strict.
* A name separator is literal (RFC 9553 section 2.2.1.2), so `", "` must carry its own space.
* A name typed on one line is never split into given and surname (wrong for many names); components come from a form.
* Problems are codes (`TypeProblem::code`, translation key suffix `problem.<code>`), never sentences (ADR 0057, `coding.md` §2a).
* Labels in a description are translation keys: lower-case words of letters and digits joined by `.` or `-`.

## Commands, change and supported (decided 2026-11-10, written in `types.capnp`)

* **`supported` per command** (`CommandDescription.supported @5`): a description lists every command the type is meant to have; one the core lacks has `supported = false` (`CommandDescription::unsupported()`), a frontend shows it locked and frees it when the core reports it. Unset on the wire reads as false (the safe side), so a writer must set it; the builder default is true.
* **`change`** = `Edit` = list of `FieldEdit` (path, basic type, values). Missing field = unchanged; no values = cleared; else replaced (`Edit::apply`). `Edit::check` validates against the description (unknown field, wrong type, several values for a single field, clearing a required field, duplicate path, rule of the basic type) and returns canonical values. `FieldEdit::pointer` gives the PatchObject key (RFC 8984 §1.4.9: steps joined by `/`, escaped as RFC 6901, no leading `/`). Values empty after trimming count as not given. The record id travels in the program's call, not in the `Edit`. A `change` command has no input fields (`ChangeWithInput`). Paths deeper than one step are `Unsupported` in `check` (`RecordProblem::NestedPath`): descriptions have flat fields.
* **`readOnly` per field** (`FieldDescription.readOnly @7`, `FieldDescription::read_only()`): the program sets it itself (uid, created, updated, prodId); a generated form shows it as display only; `Edit::check` refuses it (`RecordProblem::ReadOnly`). Unset = editable. `check_record` does not demand a required read-only field and does not refuse a value for it (an import may bring a uid); the program decides. A provider must check `change` against the description with `Edit::check`, the `Edit` alone cannot tell.
* **list = find**: `CommandDescription::standard_find` (input `term`, empty = list; paging with the `Cursor` of `common.capnp` in the contract call).
* **Record commands** (`TypeDescription::record_commands`): input has a `reference` to the type itself (or without target), plus `change` and `delete`; `type_commands` is the rest.

## JSContact and vCard: free crate checked (task point 4)

Candidate `calcard` 0.3.14 (Stalwart Labs; `Apache-2.0 OR MIT`; first released 2025-05, updated 2026-09-15, 15 releases in 2026-08/09, about 89000 downloads; repository `stalwartlabs/calcard`). Described as "iCalendar/JSCalendar and vCard/JSContact parsing, building and conversion library". Tried in a throw-away crate (not in this repo): parse a vCard 4.0 text, `VCard::into_jscontact::<String, String>()` gives a JSContact JSON value (`name.components`, `emails`, `phones`, `addresses`, `vCard.convertedProperties` for what vCard keeps), and `JSContact::into_vcard()` writes vCard 4.0 back (`FN`, `N`, `EMAIL`, `TEL;VALUE=URI`, `ADR` with `JSCOMPS`, `PROP-ID`). Other crates seen: `jscontact` 0.2.1 (RFC 9553 types only, 7000 downloads, last 2025-01), `vcard` 0.6.0 (vCard 4.0 RFC 6350 and 9554, parse and generate, no JSContact), `vcard4` 0.7.3 (parser only). **Result: use `calcard` for vCard import and export in `fs-people` (thread P); do not build our own.** Not done here: no dependency added to this repo (nothing in `fs-types` needs it), the field-by-field coverage of RFC 9553 against `description.md` is thread P's check; `calcard` pulls in more than we need (iCalendar), accepted for now; its types are generic over the id type, `String` works.

## State

| Area | State |
|---|---|
| `types.capnp` draft 0.1: `BasicType` (17 values), `Name`, `Address`, `Image` and their kinds, type description (`FieldDescription`, `CommandDescription`, `TypeDescription`, `CommandKind`), `TypeCatalog.describe` | done, draft |
| Rules for all 16 kinds: check, normalise (suggest where it makes sense: e-mail domains, language, link scheme, reference, keywords, choice) | done, tested (each kind, incl. the cases the independent review found) |
| `BasicRules` registry, unknown kind as text | done, tested |
| Description: builders, validation, registry, record check, wire round trip, `CatalogServer` (feature `rpc`) | done, tested (incl. a real RPC call) |

## Unsupported (named, not faked)

* `Edit` paths deeper than one step (map key, then property): the wire form carries them, `Edit::check` refuses them (`NestedPath`) because descriptions have flat fields; nested typing comes with nested descriptions.

* `onlineService`, `timeZone`, `duration`, `recurrence`, `quantity`, `money`, `colour`, `sound`, `video`, formatted text: not yet in `BasicType`.
* Date-time without offset (JSCalendar `LocalDateTime`) and IANA time zones: need the tz database.
* Language: well-formed only, the IANA subtag registry is not consulted. Address: country code checked for shape, not against the ISO 3166-1 list. Place: points only (no `GeoJSON` shapes; unknown `geo:` parameters are dropped; the poles are not canonicalised). Key: OpenPGP fingerprints only (no SSH, age, X.509). Phone and e-mail: no existence check (a network matter).
* Suggestions: phone, date, date-time, place, key, name, address, image, text offer none (`suggest` returns an empty list on purpose).
* Components (`input`, `display`, `compact`, `cell`) per basic type are thread C's, in `fs-components`; nothing of the look is here.
