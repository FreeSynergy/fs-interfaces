# fs-files contract — DRAFT 0.2 (2026-11-06)
#
# STATUS: DRAFT, PROVISIONAL. Owner program: fs-files. Lives in fs-interfaces (ADR 0045).
# Consumers so far: none (0 of 2). No file frontends have been analysed yet
# (docs/en/programs/apps/files/requirements.md): the target will get sharper before 1.0.
#
# What we know about a file is an object in fs-store; where its bytes are is a backend behind
# `BlobBackend`. Programs refer to a file by its id and read through `C.Blob`; they never see paths
# or backends. Saving is not exporting (ADR 0046). Evolution rules: never renumber, only append.
#
# NAMES. The `Envelope.name` of a file and of a folder IS its real name: the name a person gave it
# and, on a backend that keeps real names (the local folder), the name on the disk. It is not
# derived from anything else (ADR 0032 makes the envelope always carry a name; for files the name
# is the content). Two entries in one folder never have the same name, compared exactly; whether
# `Foto.jpg` and `foto.jpg` can sit side by side is up to the operating system. A name is refused
# if it is empty, longer than 255 bytes, `.` or `..`, contains `/`, `\` or a control character, or
# is `.fs-files` (the program's own folder). Nothing here ever replaces a file silently.
#
# PROBLEMS. Besides `unsupportedProblemType` (common.capnp) the methods below answer these RFC 9457
# problem types (`Problem.type`, with `status` and a `detail` that names the entry):
#   nameConflictProblemType     409  the name is taken in the folder, or on the disk (also a clash
#                                    that only the disk sees, e.g. a case-insensitive file system)
#   invalidNameProblemType      422  the name or path is not acceptable, here or on the disk
#   changedOutsideProblemType   409  the bytes were changed or removed outside the program; `rescan`

@0xc5c37233d97e6465;

using C = import "common.capnp";
using Uuid = C.Uuid;
using Cursor = C.Cursor;
using Timestamp = C.Timestamp;
using Envelope = C.Envelope;
using Blob = C.Blob;
using SupportReport = C.SupportReport;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;

const nameConflictProblemType   :Text = "https://freesynergy.net/problems/files/name-conflict";
const invalidNameProblemType    :Text = "https://freesynergy.net/problems/files/invalid-name";
const changedOutsideProblemType :Text = "https://freesynergy.net/problems/files/changed-outside";

# What to do when the target name is taken, for rename, move, copy and upload. `refuse` (the default, 0)
# answers a name-conflict Problem and changes nothing. `keepBoth` gives the INCOMING entry a free name:
# `name (2).ext`, `name (3).ext` (a copy: `name (copy).ext`). There is no "replace": nothing here ever
# overwrites a file, with or without a flag. Parameters added at the end of existing methods, so old
# callers send 0 = refuse (note: the Rust `copy_file` without `_as` keeps the old keep-both behaviour).
enum OnConflict { refuse @0; keepBoth @1; }

# Sort order of `list` and `find`. Ties are broken by name, then id. Unset = name ascending.
# `modified` = the time the record last changed (rename, move, new bytes found by rescan), not the disk's mtime.
# `size`: a folder counts as 0. `mediaType`: ascending only (descending answers Unsupported); a folder counts as inode/directory.
enum SortBy { name @0; modified @1; size @2; mediaType @3; }
struct Sort { by @0 :SortBy; descending @1 :Bool; }

struct File {
  envelope  @0 :Envelope;          # AS2 `Document` (`Image`, `Video`, `Audio` where they apply); envelope.name = the real file name
  mediaType @1 :Text;
  size      @2 :UInt64;
  checksum  @3 :Text;              # "blake3:<hex>"
  folder    @4 :Uuid;
  trashed   @5 :Bool;
}

struct Folder {
  envelope @0 :Envelope;           # AS2 `Collection`; envelope.name = the real folder name
  parent   @1 :Uuid;               # unset = top level
}

struct Entry {
  union {
    file   @0 :File;
    folder @1 :Folder;
  }
}

struct EntryPage { items @0 :List(Entry); next @1 :Cursor; }

# all members optional; combined with AND
struct FileFilter {
  name      @0 :Text;              # substring
  mediaType @1 :Text;              # prefix, e.g. "image/"
  folder    @2 :Uuid;
  trashed   @3 :Bool;              # true = search the trash instead, so a frontend can offer restore
  minSize   @4 :UInt64;            # bytes, inclusive; a folder counts as 0
  maxSize   @5 :Int64 = -1;        # bytes, inclusive; -1 = no limit
  modifiedAfter  @6 :Timestamp;    # inclusive; the record's last change (see SortBy.modified); unset = no limit
  modifiedBefore @7 :Timestamp;    # exclusive
  createdAfter   @8 :Timestamp;    # inclusive; when the file was created or taken over by a rescan
  createdBefore  @9 :Timestamp;    # exclusive
}

struct BackendInfo {
  id     @0 :Text;
  kind   @1 :Text;                 # local, s3, ...
  active @2 :Bool;
}

interface Files {
  supportReport @0 () -> (report :SupportReport);
  folders       @1 () -> (folders :Folders);
  transfer      @2 () -> (transfer :Transfer);
  search        @3 () -> (search :Search);
  backends      @4 () -> (backends :Backends);
  sharing       @5 () -> (sharing :Sharing);                                     # placeholder
  subscribe     @6 (listener :ChangeListener) -> (subscription :Subscription);   # file.subscribe
}

interface Folders {
  list         @0 (folder :Uuid, cursor :Cursor, sort :Sort) -> (page :EntryPage);           # folder.list
  createFolder @1 (parent :Uuid, name :Text) -> (folder :Folder);                # folder.create
  renameFolder @2 (id :Uuid, name :Text, onConflict :OnConflict) -> (folder :Folder);                    # folder.rename
  deleteFolder @3 (id :Uuid) -> ();                                              # folder.delete
  moveFile     @4 (id :Uuid, folder :Uuid, onConflict :OnConflict) -> (file :File);                      # file.move
  copyFile     @5 (id :Uuid, folder :Uuid, onConflict :OnConflict) -> (file :File);                      # file.copy
  renameFile   @6 (id :Uuid, name :Text, onConflict :OnConflict) -> (file :File);                        # file.rename
  trash        @7 (id :Uuid) -> ();                                              # file.delete
  restore      @8 (id :Uuid, onConflict :OnConflict) -> (file :File);                                    # file.restore
  purge        @9 (id :Uuid) -> ();                                              # file.purge
  get          @10 (id :Uuid) -> (found :Bool, file :File);                      # file.resolve
  classify     @11 (id :Uuid, categories :List(Text)) -> (file :File);           # file.classify (ADR 0029)
  moveFolder   @12 (id :Uuid, parent :Uuid, onConflict :OnConflict) -> (folder :Folder);                 # folder.move: into itself or below is refused
  restoreTo    @14 (id :Uuid, folder :Uuid, onConflict :OnConflict) -> (file :File);  # file.restoreTo: another folder than the original; unset folder = top level
  emptyTrash   @15 () -> (report :TrashReport);                                  # file.emptyTrash: purge everything in the trash, for good
  getFolder    @16 (id :Uuid) -> (found :Bool, folder :Folder);                  # folder.resolve
  rescan       @13 (folder :Uuid) -> (report :RescanReport);                     # folder.rescan: see RescanReport. PROVISIONAL rules; unset folder = the top
}

# What `emptyTrash` did. A file that cannot be purged stays in the trash and is listed in `failed`;
# the others are still purged.
struct TrashReport {
  purged @0 :UInt32;
  failed @1 :List(Uuid);
}

# What `rescan` found. It deletes nothing and overwrites nothing: files and folders that are on the disk
# but unknown are taken over (`added`); files whose size or modification time changed outside get a new
# size and checksum (`updated`; also bytes found in the trash folder after a crash and put back); files
# without bytes are only reported (`missing`) and their records are kept; `skipped` says why an entry
# was not taken over (symlink, name not accepted, name used by a record on another backend, ...).
struct RescanReport {
  added   @0 :List(Entry);
  updated @1 :List(File);
  missing @2 :List(File);
  skipped @3 :List(Text);
}

# resumable: an upload survives a lost connection; ask `received` and go on from there
interface Upload {
  write    @0 (offset :UInt64, data :Data) -> ();
  received @1 () -> (bytes :UInt64);
  finish   @2 () -> (file :File);
  abort    @3 () -> ();
}

# An unfinished upload, as listed by `Transfer.pendingUploads`.
struct UploadInfo {
  id        @0 :Text;              # for `resumeUpload`
  folder    @1 :Uuid;              # unset = top level
  name      @2 :Text;
  mediaType @3 :Text;
  received  @4 :UInt64;            # bytes the backend holds: where `write` goes on
  started   @5 :Timestamp;
}

interface Transfer {
  startUpload  @0 (folder :Uuid, name :Text, mediaType :Text, onConflict :OnConflict) -> (upload :Upload, id :Text);  # file.upload
  resumeUpload @1 (id :Text) -> (upload :Upload);                                # file.upload
  open         @2 (id :Uuid) -> (blob :Blob);                                    # file.download
  pendingUploads @3 () -> (items :List(UploadInfo));                             # file.upload: unfinished uploads, oldest first
}

interface Search {
  find @0 (filter :FileFilter, cursor :Cursor, sort :Sort) -> (page :EntryPage);             # file.search
}

interface Backends {
  list   @0 () -> (items :List(BackendInfo));                                    # backend.list
  add    @1 (config :Text) -> (info :BackendInfo);                               # backend.add: checked, then swapped in
  remove @2 (id :Text) -> ();                                                    # backend.remove
}

# PLACEHOLDER: sharing is one specification for all programs, not written yet (design.md, question 3).
interface Sharing {
}

# ---- Provider interface: where bytes are. Local folder, S3 (RustFS and others), later WebDAV. ----

struct BlobInfo {
  size     @0 :UInt64;
  checksum @1 :Text;               # may be empty from `stat`: hashing a whole blob to answer a size question is wrong
  modified @2 :Int64;              # last change, ns since the epoch; 0 = the backend has none (S3). A rename keeps it.
}

interface BlobBackend {
  supportReport @0 () -> (report :SupportReport);
  put    @1 (key :Text, offset :UInt64, data :Data) -> ();                       # blob.write
  commit @2 (key :Text) -> (info :BlobInfo);                                     # blob.write; on a backend that keeps real names it never replaces (name-conflict)
  get    @3 (key :Text, offset :UInt64, length :UInt32) -> (data :Data);         # blob.read; may answer fewer bytes than asked, empty = the end
  stat   @4 (key :Text) -> (found :Bool, info :BlobInfo);                        # blob.read
  delete @5 (key :Text) -> ();                                                   # blob.delete
  pending @6 (key :Text) -> (bytes :UInt64);                                     # blob.write: how much of an unfinished key is held, for resuming

  # ---- backends that keep real names (local folder). Others answer `Unsupported` for these. ----
  naming  @7 () -> (byName :Bool);                                               # true: keys are paths of real names, folders exist on the disk
  publish @8 (pending :Text, key :Text) -> (info :BlobInfo);                     # blob.rename: pending blob -> path `key`, atomic, NEVER replaces (name-conflict)
  relocate @9 (from :Spot, to :Spot) -> ();                                      # blob.rename: file or folder, NEVER replaces (name-conflict); missing source = notFound
  discard @10 (spot :Spot) -> ();                                                # blob.delete: missing is fine
  makeDir @11 (path :Text, parents :Bool) -> ();                                 # blob.rename: without `parents` an existing path is a name-conflict
  removeDir @12 (path :Text) -> ();                                              # blob.rename: only an empty folder; missing is fine
  scan    @13 (dir :Text) -> (items :List(Scanned));                             # blob.scan: one level, sorted by name; empty = the top
  has     @14 (spot :Spot) -> (found :Bool);                                     # blob.read: is something at this spot (bytes left in the trash by a crash)
  sweepPending @15 (keep :List(Text), olderThanSecs :UInt64) -> (removed :UInt32); # blob.delete: pending blobs not in `keep` and older than that: leftovers of a crash
}

# Where a blob is. `pending` and `trash` are inside the program's own hidden folder: never a user file.
struct Spot {
  union {
    path    @0 :Text;              # committed blob or folder: a path of real names, `/`-separated
    pending @1 :Text;              # unfinished upload with this key
    trash   @2 :Text;              # trash entry with this id
  }
}

enum ScanKind { file @0; dir @1; skipped @2; }

struct Scanned {
  name       @0 :Text;
  kind       @1 :ScanKind;
  size       @2 :UInt64;           # files only
  modified   @3 :Int64;            # ns since the epoch, 0 = unknown
  skipReason @4 :Text;             # when kind = skipped
}
