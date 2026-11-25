# Courseware

This repository contains three course packages and a portable Rust adapter that opens them. A host can list each book by its declared title, display its Contents and present the reading or exercise workspace requested by the course.

| Package | Source and workspace |
| --- | --- |
| [The Rust Book](courses/rust-book/course.toml) | Brown CEL’s interactive Rust Book, including ordered chapters, quizzes and Rust listings. |
| [The Mechanics of Proof](courses/mechanics-of-proof/) | Heather Macbeth’s proof text and a Lean 4 exercise from the pinned edition. |
| [CS2023 Topic Map](courses/cs2023/) | Original study prompts and citations mapped to selected CS2023 knowledge units. |

## Build and test

From a checkout of this repository, initialize the textbook submodules and run the Cargo workspace tests with a stable Rust toolchain supporting Edition 2024.

```sh
git submodule update --init --recursive
cargo test --locked
```

Cargo fetches dependencies from crates.io on the first build. Once cached, `cargo test --locked --offline` runs against the same package versions. The managed Qrate unit is `courseware.adapters/courseware-adapters`.

## Open a reading

```rust
use courseware_adapters::Course;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let course = Course::open(Path::new("courses/rust-book"))?;
    let opening = course.chapter(&course.workspace.default_chapter).unwrap();
    println!("{} — {}", course.tablet.title, opening.title);
    Ok(())
}
```

The [adapter specification](adapters/spec.md) describes the portable data, package tables, host integration and optional protobuf bindings. The Rust Book’s [data notice](adapters/data/NOTICE.md) identifies the source of the included permission illustrations and how to reproduce them. Each textbook submodule retains its upstream copyright and licensing materials.
