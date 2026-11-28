# Courseware adapters 0.5

The adapter opens a course at a chosen source revision and presents its chapters, Contents and workspace settings to a host. A host supplies navigation URLs, rendering, progress storage and permission to run exercises. The course package owns the titles, reading order, exercise identities and checking rules.

| Interface | Version | Purpose |
| --- | --- | --- |
| `courseware.adapters/courseware-adapters` | 0.5.2 | Adapt four declared course editions into portable reading data. |
| `qn.ux.v05.TreeNavigation` | 0.5 | Names the presentation requested by the example manifests. A host maps navigation nodes to its own links. |

## Package declaration

The [public schema](schema/courseware.proto) describes the declaration and reading data independently of the rendering host. Each package supplies `course.toml` with `format_version = 1`, an adapter `profile`, and a source `revision`. The `[tablet]` table declares a stable ID and title. The `[navigation]` table gives Contents its own ID, title, presentation kind and source. The adapter derives ordered roots and children. Each destination references a chapter ID within the course. The `[workspace]` table names a language, the `exercise` or `reading` editor mode and the initially selected chapter. Original format-1 CEL packages can omit these tables and receive the Rust Book defaults.

`cel-rust-book` reads the pinned source commit, listings and quiz definitions. `mechanics-of-proof` reads its pinned textbook and declared Lean exercise. `cs2023` verifies the SHA-256 of its local authored topic map. The CS2023 map identifies its official curriculum and companion texts through citations. The declared revision belongs to the profile's source format.

## Executable and protobuf interfaces

The `courseware.adapters` 0.5.2 Qrate provides the `validate-learning-map` executable. It opens the pinned edition and keeps its Rust types, source loader and grading keys inside the process. Consumers invoke the declared `prepare-import` or `archive-session` action and decode the operation’s public wire format. The executable reads the same human-authored TOML and LearningRoot JSON as the preceding release.

`CourseReading` carries reader-visible content through the archive session. Ordered blocks represent Markdown, code, interactions, notices or steps. The [wire fixture](tests/fixtures/course-reading.textproto) exercises nested navigation, ordered blocks, explicit starter presence and a question reference. Run `python3 adapters/tools/check_schema.py` with an Edition 2023 capable `protoc` on PATH or set `PROTOC` to its executable.

## Adding a course profile

Add an adapter for the source format, register its profile in `Course::open_with_git`, and add a package manifest with attributable source data. Tests should verify the declared revision and default chapter, renderable reading order and the association between interactions and their checking data. Profile-specific exercises can place additional validated declarations beside the manifest. Existing hosts can render any profile that stays within their supported block and workspace types.

## Opening a passage by wire

Run `validate-learning-map ROOT_JSON COURSES_DIR MAP_TOML --adapted-course ALIAS` to obtain the Learning passage order and exercise identities at the pinned edition. The CEL edition contributes each public quiz question as an exercise. Outcome, checker and completion-rule references are populated when explicitly declared by the root.

For a selected passage, `--chapter-posit ALIAS CHAPTER_ID` emits a binary `qune.core.Posit`. Its four edits write the ID, title, source revision and ordered Tablet blocks to the ThingRef in that passage’s Learning CellRef. Each block uses the Tablet owner’s message type. Given a CEL interaction ID, `--question-wire ALIAS QUESTION_ID` emits the public `courseware.v05.Question` from the same pinned edition. Choice, tracing and short-answer prompts include the original public starter when available. Rendering referenced image assets and grading or revealing answers still require the adapter's respective operations.

For an owner-issued short-answer check, `validate-learning-map ROOT COURSES MAP --question-wire ALIAS ID` emits the public `courseware.v05.Question`, and `--question-cell ALIAS ID` emits the canonical pinned Learning CellRef. With those verified, `--check-question ALIAS ID PUBLIC_QUESTION_SHA256` accepts a binary `google.protobuf.StringValue` on standard input (up to 64 KiB) and emits a binary `google.protobuf.BoolValue`. Compare the exit status first. A successful incorrect answer has the protobuf encoding of false, which can be empty. Unknown questions, different prompt digests, malformed requests and other question types fail. The grading key stays in the pinned course adapter.


To publish a whole edition, use `--course-posit ALIAS`. The command opens the pinned catalog once and emits one unframed core Posit. It writes AdaptedCourse at the tablet revision and all chapters in reading order at their respective cell identities. Repeating the command with identical admitted inputs yields identical bytes. The connected-lessons overlay emits the lessons in manifest order. Each exercise cell carries a Chapter with its prompt, exact Rust and Lean starter contents, attribution and further reading. Its edition identity binds the source manifest and all referenced starter and check bytes.

From the repository containing the qm checkout, the managed command writes raw protobuf to stdout. Paths resolve from the invoking directory.

```sh
courses=qm/courseware/courses
qm run qm/courseware/adapters prepare-import \
  "$courses/connected-lessons/root.json" "$courses" \
  "$courses/connected-lessons/learning-map.toml" --course-posit rust-book > rust-book.posit
```

Use `--root-posit` in the same managed command to publish the catalog itself. The target is the declared LearningRoot identity. Its seven typed edits include that identity, course and map pins, topics, outcomes, exercises, and the alias-to-ThingRef map. `--root-wire` emits the original LearningRoot message.

### Reissuing an edition after a producer upgrade

The adapter release version participates in `AdaptCourseRequest.adapter`. Publishing identical source with 0.5.2 retains the catalog, map, passage and exercise identities. Each AdaptedCourse records the newly selected producer, which changes the edition Posit and its admitted cut. Keep the previous receipt and re-enroll readers at the new cut when switching producers. An operator can run the following comparison with the original course checkout and its first-publication receipt:

```sh
python3 adapters/tests/compare_production.py OLD_ADAPTER NEW_ADAPTER COURSES PUBLICATION_JSON LEARNING_FDS
```

The check verifies all six original Posit hashes, compares catalog and map bytes, and compares every edition edit and selected producer basis.

### Cross-references at the published CEL edition

In Rust Book revision `88250e0392cef0622f318e35469108d68694c1f7`, 184 Markdown link occurrences in the published chapters have a relative filename matching a published chapter ID after removing its extension. The Markdown stores the original URL. A reader can build a lookup from chapter ID to the corresponding published cell. Ten links contain only a fragment. One link in `ch04-03-fixing-ownership-errors` requires the pinned mdBook redirect from `ch19-01-unsafe-rust.html` to `ch20-01-unsafe-rust.html`. Another points to the separately hosted standard library at `../std/index.html`. The edition contains neither the redirect table nor that library.

All 22 local Markdown images exist in the pinned upstream Git tree. For example, `ch15-01-box` references `img/trpl15-01.svg`. Each appears as a relative path in a Markdown block. Display from the published data requires a resolver with access to the pinned assets. Tablet Chapter has no field for an asset body or typed hyperlink target. A Learning SourceAnchor can describe a source file with a path, selector and digest.

### Reading a fixed edition from another process

The managed `courseware.adapters archive-session` command serves
`courseware.archive.v1.Reader` over local standard input and output. Its
[protobuf](schema/archive.proto) and [import-complete descriptor](schema/archive.fds)
describe the frames. An Open selects a standalone package directory or a connected
root. The owner validates the package pins, adapts chapters and keeps the opened
edition alive until the caller closes the connection. The reply carries
`courseware.v05.CourseReading` for each course. A connected Open also returns
validated placements and the pinned LearningRoot in protobuf JSON.

Queries select the opened alias and exercise or asset. The owner checks answers
and releases explanations on request. A host is responsible for permitting
a reveal for the current learner. Reading frames contain the public prompt and
choices. Course answers and checker rules remain in the adapter. A lesson-resource
query can retrieve an explicitly pinned resource for the archived connected
exercise checker. Callers must keep that operator interface private.

Hosted Open uses the caller's immutable Git archive reader. A refused archive
ends Open. The adapter retains the resulting course for subsequent grading and
resource lookup. Each frame starts with a four-byte, big-endian body length
and is limited to 128 MiB. Run `adapters/tests/archive_session.py BINARY
courses/rust-book` with Python protobuf to exercise the actual package.
