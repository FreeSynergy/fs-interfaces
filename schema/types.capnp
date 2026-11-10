# FreeSynergy basic types and type description - DRAFT 0.1 (2026-11-10)
#
# STATUS: DRAFT (ADR 0045, ADR 0066). Owner: the shared types layer (Rust crate `fs-types` in this repo).
# Consumers so far: none (0 of 2); planned: fs-people (Person), the generated admin frontend.
#
# Two parts:
#   1. BASIC TYPES (ADR 0066 point 1, docs/en/programs/map.md): small values with fixed rules and one standard each.
#      Defined once, valid in every program. The rules (check, normalise, suggest) live in `fs-types`;
#      this file holds the wire form and names the standard of each type.
#   2. TYPE DESCRIPTION (ADR 0066 point 3): what a program tells the world about a data type it owns: fields (name, basic type,
#      required, multiple, label as translation key) and commands (name, input, result). Not the look, only what is possible.
#      A frontend generated from it needs no knowledge of the program. Every provider offers `TypeCatalog.describe`.
#
# Evolution rules (Cap'n Proto): never renumber or remove; only append. Ordinal 0 of an enum is `unknown`: a reader that
# meets a newer value reads `unknown` and treats the field as plain text.
@0x916fad50f1c14440;

# ---------------------------------------------------------------- 1. basic types

# The kinds of basic type. The standard of each is in the comment (ADR 0014: the standard is the trait).
enum BasicType {
  unknown       @0;
  text          @1;    # one line of text, Unicode NFC, trimmed
  multilineText @2;    # several lines of text, same rules
  email         @3;    # RFC 5322 addr-spec (practical rule), JSContact EmailAddress; domain in IDNA (RFC 5891) form
  phone         @4;    # E.164 (ITU-T), JSContact Phone; stored as +<digits>
  name          @5;    # JSContact Name (RFC 9553 section 2.2.1.1): `full` and/or `components`
  address       @6;    # JSContact Address (RFC 9553 section 2.5.1.1): components, countryCode (ISO 3166-1 alpha-2), full
  place         @7;    # geo: URI (RFC 5870): latitude, longitude, optional altitude and uncertainty; WGS-84
  date          @8;    # RFC 3339 full-date (YYYY-MM-DD)
  dateTime      @9;    # RFC 3339 date-time with offset (instant)
  language      @10;   # BCP 47 (RFC 5646), normalised case
  link          @11;   # RFC 3986 URI (JSContact Link, `uri`); an IRI (RFC 3987) is accepted and kept as typed
  image         @12;   # AS2 Image: a link whose media type is image/* (+ size when known)
  key           @13;   # OpenPGP fingerprint (RFC 9580: 40 or 64 hex digits), JSContact CryptoKey names it by URI
  choice        @14;   # one entry of a fixed list (JSCalendar enums, own lists); the list is in the field description
  keywords      @15;   # set of short words, JSContact/JSCalendar `keywords`
  reference     @16;   # AS2 `id` (IRI) of an object; a frontend shows the target's compact component
  # Append new kinds here (onlineService, timeZone, duration, recurrence, quantity, money, colour, sound, video,
  # formatted text ... are in map.md and still to come).
}

# Wire forms of the structured basic types. Single-valued basic types (email, phone, date, ...) travel as `Text`
# in canonical form, as the checking function returns it.

struct NameComponent {
  kind  @0 :NameKind;
  value @1 :Text;
}
# JSContact NameComponent kinds (RFC 9553 section 2.2.1.2).
enum NameKind { unknown @0; title @1; given @2; given2 @3; surname @4; surname2 @5; credential @6; generation @7; separator @8; }

struct Name {
  full       @0 :Text;                  # may be empty when components are given
  components @1 :List(NameComponent);   # in the order of the full name
}

struct AddressComponent {
  kind  @0 :AddressKind;
  value @1 :Text;
}
# JSContact AddressComponent kinds (RFC 9553 section 2.5.1.2), the ones common to all countries.
enum AddressKind { unknown @0; room @1; apartment @2; floor @3; building @4; number @5; name @6; block @7; subdistrict @8; district @9; locality @10; region @11; postcode @12; country @13; direction @14; landmark @15; postOfficeBox @16; separator @17; }

struct Address {
  components  @0 :List(AddressComponent);
  countryCode @1 :Text;                  # ISO 3166-1 alpha-2, upper case; empty = not given
  full        @2 :Text;                  # the address as one text; may be empty when components are given
}

struct Image {
  uri       @0 :Text;
  mediaType @1 :Text;                    # image/..., empty = not known
  width     @2 :UInt32;                  # pixels, 0 = not known
  height    @3 :UInt32;
}

# ---------------------------------------------------------------- 2. type description

# What a command does to the data; a generated frontend picks the button and the confirmation from it.
enum CommandKind { unknown @0; create @1; change @2; find @3; delete @4; check @5; suggest @6; other @7; }

# One field of a data type, or one input or result of a command.
struct FieldDescription {
  name     @0 :Text;                     # machine name: the JSContact property, e.g. "emails"; lower camel case, unique per type
  basic    @1 :BasicType;
  required @2 :Bool;
  multiple @3 :Bool;                     # a list or set of values
  label    @4 :Text;                     # translation key (coding.md section 2a), never a sentence: "person.emails"
  target   @5 :Text;                     # for `reference`: name of the data type referred to; empty = any
  choices  @6 :List(Text);               # for `choice`: the allowed keys (a label key per entry is "<label>.<key>")
}

struct CommandDescription {
  name   @0 :Text;                       # machine name, unique per type: "create", "patch", "find"
  kind   @1 :CommandKind;
  label  @2 :Text;                       # translation key
  input  @3 :List(FieldDescription);
  result @4 :List(FieldDescription);     # empty = nothing but success; `multiple` = a list
  # Whether the program can do it now. The description lists EVERY command the type is meant to have; one the core does not
  # have yet is reported with `supported = false` (the provider rule of ADR 0045 made visible). A frontend shows it locked
  # ("coming"), and frees it when the core reports it, without a change to the frontend. A writer MUST set it: unset
  # reads as false, i.e. "not supported" (the safe side).
  supported @5 :Bool;
}

# RULES FOR COMMANDS (decided 2026-11-10, same for every type and program):
#   list   = `find`. There is no list command per program: a list is `find` with an empty term; it pages with a `Cursor`
#            (common.capnp) the same way for all types. Filters beyond `term` are further input fields of that `find`.
#   change = kind `change`. Its input is an `Edit` (below) and nothing else (the command's `input` list is empty): field
#            path plus typed value, field by field. The backend turns it into the standard's patch (e.g. a JSContact or
#            JSCalendar PatchObject); no JSON appears in this contract or in a form.
#   record commands = the commands that work on ONE record: those whose input holds a `reference` field with `target`
#            equal to the type's own name (or empty target), plus `change` and `delete`. A frontend puts these on the
#            record (menu, toolbar); the others (create, find, ...) belong to the type as a whole.

# One field of a record changed by a `change` command (decided 2026-11-10).
# RULE: a field that is not in the `Edit` stays as it is. A field in the `Edit` with no values (`values` empty, the null of a
# PatchObject) is cleared. Otherwise `values` replaces all values of the field; for a single-valued field it holds exactly one.
struct FieldEdit {
  path   @0 :List(Text);                 # the field: ["emails"]; deeper steps (a map key, then a property) are further entries
  basic  @1 :BasicType;                  # the basic type of the values, so the backend can check without the description
  values @2 :List(Text);                 # canonical texts as the rules of `fs-types` return them (Name/Address/Image: their one-line form)
}

struct Edit {
  fields @0 :List(FieldEdit);            # at most one entry per path
}

struct TypeDescription {
  name     @0 :Text;                     # data type name, unique across programs: "Person"
  program  @1 :Text;                     # owner program (ADR 0050): "fs-people"
  standard @2 :Text;                     # the standard it follows: "RFC 9553 Card"
  label    @3 :Text;                     # translation key of the type
  fields   @4 :List(FieldDescription);
  commands @5 :List(CommandDescription);
}

# Every program that owns a data type implements this (the provider rule of ADR 0045 applies:
# a program with no data type of its own answers an empty list).
interface TypeCatalog {
  describe @0 () -> (types :List(TypeDescription));
}
