# Courseware

This repository contains four course editions and a portable Rust adapter that opens them. A host can list each book by its declared title, display its Contents and present the reading or exercise workspace requested by the course.

| Package | Source and workspace |
| --- | --- |
| [The Rust Book](courses/rust-book/course.toml) | Brown CEL’s interactive Rust Book, including ordered chapters, quizzes and Rust listings. |
| [The Mechanics of Proof](courses/mechanics-of-proof/) | Heather Macbeth’s proof text and a Lean 4 exercise from the pinned edition. |
| [CS2023 Topic Map](courses/cs2023/) | Original study prompts and citations mapped to selected CS2023 knowledge units. |
| [Connected Lessons](courses/connected-lessons/lessons.toml) | Authored exercises and explicit links across the other three editions. |

## Build and test

From a checkout of this repository, initialize the textbook submodules and run the Cargo workspace tests with a stable Rust toolchain supporting Edition 2024.

```sh
git submodule update --init --recursive
cargo test --locked
```

Cargo fetches dependencies from crates.io on the first build. Once cached, `cargo test --locked --offline` runs against the same package versions. The managed Qrate unit is `courseware.adapters/courseware-adapters`.

## Open a reading

The adapter command emits a `qune.core.Posit` containing the selected edition and its ordered Tablet Chapters. From the repository root:

```sh
cargo run --locked --bin validate-learning-map -- \
  courses/connected-lessons/root.json courses courses/connected-lessons/learning-map.toml \
  --course-posit rust-book > rust-book.posit.pb
```

The [adapter specification](adapters/spec.md) describes the portable data, package tables, host integration and optional protobuf bindings. The Rust Book’s [data notice](adapters/data/NOTICE.md) identifies the source of the included permission illustrations and how to reproduce them. Each textbook submodule retains its upstream copyright and licensing materials.

### Opening the published courses

The [Rust Book package](courses/rust-book/Qrate.toml) opens **Hello, World!**, the starting passage selected in its course declaration. The [connected course collection](courses/Qrate.toml) opens **The `match` Control Flow Construct**, where a published map can lead to *Images and preimages*. Each Qrate names a serialized `qune.compose.Skene` artifact. Its readable `entry.skene.textproto` source records the Chapter and Actions Thing and the exact compatible reader schemas. The entries reference passages from the pinned CEL edition. They do not contain local paths, learner data, executable grants or socket credentials.

After the course source and dependencies are present locally, verify both artifacts against the actual Chapter Posits and reader release with:

```sh
python3 courses/verify_entries.py \
  --adapter /path/to/validate-learning-map \
  --protoc /path/to/protoc \
  --compose-schema /path/to/import-complete-compose.fds \
  --reader-release /path/to/attributed-reader-release
```

The verifier needs Python protobuf, the initialized textbook submodules, and the matching local adapter and release. It runs offline. To regenerate an entry after editing its source, pass the owner's compose descriptor to `protoc --descriptor_set_in=… --encode=qune.compose.Skene compose.proto` and write its output to the corresponding `entry.skene.pb`. Update the artifact size and SHA-256 in the Qrate before verifying.

For opening in a desktop, an operator registers the intended checkout and first publishes its course data and Contents. Use `declared_tablet.py` with the reviewed digest of that checkout's `Qrate.toml` and its separately enrolled collection. Pass the resulting private policy as the primary enrollment to the read supervisor. Mapped follow grants belong to the connected collection. The quiz, Files and terminal each continue to require their member-specific authority at the selected Actions cut. A reader can open both entries without sharing the learner's scratch or changing the checkout scope of any grant.
