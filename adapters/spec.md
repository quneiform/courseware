# Courseware adapters 0.5

The adapter opens a course at a chosen source revision and presents its chapters, Contents and workspace settings to a host. A host supplies navigation URLs, rendering, progress storage and permission to run exercises. The course package owns the titles, reading order, exercise identities and checking rules.

| Interface | Version | Purpose |
| --- | --- | --- |
| `courseware.adapters/courseware-adapters` | 0.5.0 | Load three declared course profiles into portable reading data. |
| `qn.ux.v05.TreeNavigation` | 0.5 | Names the presentation requested by the example manifests. A host maps navigation nodes to its own links. |

## Package declaration

The [public schema](schema/courseware.proto) describes the declaration and reading data independently of the rendering host. Each package supplies `course.toml` with `format_version = 1`, an adapter `profile`, and a source `revision`. The `[tablet]` table declares a stable ID and title. The `[navigation]` table gives Contents its own ID, title, presentation kind and source. The adapter derives ordered roots and children. Each destination references a chapter ID within the course. The `[workspace]` table names a language, the `exercise` or `reading` editor mode and the initially selected chapter. Original format-1 CEL packages can omit these tables and receive the Rust Book defaults.

`cel-rust-book` reads the pinned source commit, listings and quiz definitions. `mechanics-of-proof` reads its pinned textbook and declared Lean exercise. `cs2023` verifies the SHA-256 of its local authored topic map. The CS2023 map identifies its official curriculum and companion texts through citations. The declared revision belongs to the profile's source format.

## Rust and protobuf interfaces

`Course::open` accepts a package directory. `Course::open_with_git` lets a host supply the immutable Git archive reader for a Git-backed course. The Rust API returns a `Course` with its revision, Tablet, recursive navigation, workspace and ordered chapters. The five block cases carry Markdown, source code, an interaction ID, a notice, or labeled steps. A host can look up a public question, check submitted answers, request a reveal or read an available asset. Answer keys remain with the adapter.

`CourseReading` in the standalone protobuf records reader-visible data. Its questions correspond to public question lookups. The in-process crate exposes Rust records. Consumers needing binary interchange can generate their own binding from the schema. The [wire fixture](tests/fixtures/course-reading.textproto) exercises nested navigation, ordered blocks, explicit starter presence and a question reference. Run `python3 adapters/tools/check_schema.py` with an Edition 2023 capable `protoc` on PATH or set `PROTOC` to its executable.

## Adding a course profile

Add an adapter for the source format, register its profile in `Course::open_with_git`, and add a package manifest with attributable source data. Tests should verify the declared revision and default chapter, renderable reading order and the association between interactions and their checking data. Profile-specific exercises can place additional validated declarations beside the manifest. Existing hosts can render any profile that stays within their supported block and workspace types.
