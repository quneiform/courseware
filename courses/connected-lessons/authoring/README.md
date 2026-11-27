# Author a connected lesson

Work in a fresh checkout of the public courseware repository with its pinned submodules initialized. Run the commands below from the repository root. This starter leaves the learner implementation, proof, effective Rust checks, proof vocabulary and placement choices for the author.

The Lean checker uses each exercise’s declared vocabulary. Include numeric literals such as `2` in `allowed_names` when a valid proof writes them, then regenerate the root pin after changing the checker.

## Establish the baseline

```sh
git submodule update --init --recursive
overlay=courses/connected-lessons
validate() { cargo run -q --locked --bin validate-learning-map -- "$@"; }
validate "$overlay/root.json" courses "$overlay/learning-map.toml"
```

The current edition opens three real passages onto one images-and-preimages exercise. To practice reading a diagnostic independently of that edition, copy the intentionally broken fixture and run draft validation.

```sh
cp "$overlay/authoring/unresolved-passage.toml" /tmp/learning-practice-map.toml
validate "$overlay/root.json" courses /tmp/learning-practice-map.toml --draft
```

The diagnostic names the missing Rust passage and the offending TOML location. Replace its cell with a real chapter ID from the Rust Book’s pinned table of contents. Re-run the same command. A draft check reads the file and reports its result. Opening a published map additionally checks the exact map pin.

## Build induction and loop invariants

1. Copy `authoring/induction-and-loop-invariants/` to `courses/connected-lessons/induction-and-loop-invariants/`. In the Rust starter, the learner will return every intermediate tick count. Complete the author's `check.rs`. Exercise the empty trace, the base case and the step. Ensure a learner who returns the correct length or final value with incorrect intermediate values fails.
2. The Lean starter states the invariant for a recursive model of that loop. Its paired `check.toml` declares the recursive model, the theorem header, toolchain and allowed proof names. Keep the header byte-for-byte identical in the starter and check. Determine which identifiers the intended proof needs and add those to `allowed_names`. Keep the starter incomplete. Use the installed reader to try a valid proof and a placeholder before publishing.
3. Add one `[[lessons]]` to `lessons.toml`. Give it the ID `induction-and-loop-invariants`, a learner-facing title and prompt, the author attribution, and a relevant source URL. Copy the worked lesson's Rust and Lean tables and point them to the new starter and checker files. Rust checkers append the author's `fn main()` to the submitted source. Lean checkers verify the pinned theorem against the named Lean toolchain.
4. In `root-authoring.toml`, add the topic, outcome and exercise declarations, following the worked example. Give the exercise tablet `connected-lessons` and the cell ID chosen above. Reuse the declared checker and completion rule for the Rust-and-Lean pair.
5. Append independently named `[[placements]]` in `learning-map.toml`. Map the same exercise into CS2023's `induction-and-invariants`, the Rust Book's `ch03-05-control-flow`, and a chosen Macbeth section, such as `06-induction--introduction`. The `at` address identifies a real passage. The `exercise` address is shared by each placement. Set `topic` and `assesses` to the keys you declared in step 4.

## Validate the edition and try it

```sh
candidate=$(mktemp)
validate --author-root courses "$overlay/root-authoring.toml" > "$candidate" &&
  validate "$candidate" courses "$overlay/learning-map.toml" &&
  cp "$candidate" "$overlay/root.json"
validate "$overlay/root.json" courses "$overlay/learning-map.toml" --wire
```

The root binds source editions, the exact map, and the lesson starters and checks. Repeat the repin command after changing any of those inputs. Commit the reviewed package files and generated root together.

Open the overlay in a connected-lesson-capable Quneic reader with `--course courses/connected-lessons` and an independent scratch checkout. Open the exercise from two mapped passages. Check a failing and a passing Rust submission and an unfinished and passing Lean submission. Reopen the saved work and verify the same attempt and evidence. Record the commands, revisions, how long the work took, and any step that required a host change. The plain public checkout supplies the validator. Running and resuming learner work also requires the host and its pinned checker toolchains.
