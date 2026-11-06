# fs-files contract — DRAFT 0.1 (2026-11-06)
#
# STATUS: DRAFT, PROVISIONAL. Owner program: fs-files. Lives in fs-interfaces (ADR 0045).
# Consumers so far: none (0 of 2). No file frontends have been analysed yet
# (docs/en/programs/apps/files/requirements.md): the target will get sharper before 1.0.
#
# What we know about a file is an object in fs-store; where its bytes are is a backend behind
# `BlobBackend`. Programs refer to a file by its id and read through `C.Blob`; they never see paths
# or backends. Saving is not exporting (ADR 0046). Evolution rules: never renumber, only append.

@0xc5c37233d97e6465;

using C = import "common.capnp";
using Uuid = C.Uuid;
using Cursor = C.Cursor;
using Envelope = C.Envelope;
using Blob = C.Blob;
using SupportReport = C.SupportReport;
using ChangeListener = C.ChangeListener;
using Subscription = C.Subscription;

struct File {
  envelope  @0 :Envelope;          # AS2 `Document` (`Image`, `Video`, `Audio` where they apply)
  mediaType @1 :Text;
  size      @2 :UInt64;
  checksum  @3 :Text;              # "blake3:<hex>"
  folder    @4 :Uuid;
  trashed   @5 :Bool;
}

struct Folder {
  envelope @0 :Envelope;           # AS2 `Collection`
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
  list         @0 (folder :Uuid, cursor :Cursor) -> (page :EntryPage);           # folder.list
  createFolder @1 (parent :Uuid, name :Text) -> (folder :Folder);                # folder.create
  renameFolder @2 (id :Uuid, name :Text) -> (folder :Folder);                    # folder.rename
  deleteFolder @3 (id :Uuid) -> ();                                              # folder.delete
  moveFile     @4 (id :Uuid, folder :Uuid) -> (file :File);                      # file.move
  copyFile     @5 (id :Uuid, folder :Uuid) -> (file :File);                      # file.copy
  renameFile   @6 (id :Uuid, name :Text) -> (file :File);                        # file.rename
  trash        @7 (id :Uuid) -> ();                                              # file.delete
  restore      @8 (id :Uuid) -> (file :File);                                    # file.restore
  purge        @9 (id :Uuid) -> ();                                              # file.purge
  get          @10 (id :Uuid) -> (found :Bool, file :File);                      # file.resolve
  classify     @11 (id :Uuid, categories :List(Text)) -> (file :File);           # file.classify (ADR 0029)
  moveFolder   @12 (id :Uuid, parent :Uuid) -> (folder :Folder);                 # folder.move: into itself or below is refused
}

# resumable: an upload survives a lost connection; ask `received` and go on from there
interface Upload {
  write    @0 (offset :UInt64, data :Data) -> ();
  received @1 () -> (bytes :UInt64);
  finish   @2 () -> (file :File);
  abort    @3 () -> ();
}

interface Transfer {
  startUpload  @0 (folder :Uuid, name :Text, mediaType :Text) -> (upload :Upload, id :Text);  # file.upload
  resumeUpload @1 (id :Text) -> (upload :Upload);                                # file.upload
  open         @2 (id :Uuid) -> (blob :Blob);                                    # file.download
}

interface Search {
  find @0 (filter :FileFilter, cursor :Cursor) -> (page :EntryPage);             # file.search
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
}

interface BlobBackend {
  supportReport @0 () -> (report :SupportReport);
  put    @1 (key :Text, offset :UInt64, data :Data) -> ();                       # blob.write
  commit @2 (key :Text) -> (info :BlobInfo);                                     # blob.write
  get    @3 (key :Text, offset :UInt64, length :UInt32) -> (data :Data);         # blob.read
  stat   @4 (key :Text) -> (found :Bool, info :BlobInfo);                        # blob.read
  delete @5 (key :Text) -> ();                                                   # blob.delete
  pending @6 (key :Text) -> (bytes :UInt64);                                     # blob.write: how much of an unfinished key is held, for resuming
}
