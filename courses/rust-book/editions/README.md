# Rust Book with source-linked chapters

The default course manifest remains format 1. To publish a successor with whole-chapter cross-references, start with a separate recursive checkout of the public courseware repository. In that checkout run the commands below with the released adapter available as `validate-learning-map`.

```sh
cp courses/rust-book/editions/source-anchors.toml courses/rust-book/course.toml
validate-learning-map --author-root courses courses/connected-lessons/root-authoring.toml > courses/connected-lessons/root.json
validate-learning-map courses/connected-lessons/root.json courses courses/connected-lessons/learning-map.toml
```

Format 2 gives each adapted Rust Book passage a Learning SourceAnchor containing its repository-relative Markdown filename and SHA-256 digest of the original bytes at the pinned Git commit. Both formats read the same CEL content. The changed course manifest gives the successor a distinct edition and distinct chapter Things.

An operator selects the new root digest and adapter executable for ordinary project publication, then enrolls the released Learning Contents computation for that edition. Its authorized reading targets come from the edition's own collection. Previous bookmarks, checked Attempts and effect enrollments continue to name their original edition. Switching a learner's default requires a deliberate choice about migrating position and separately enrolling effects for the new chapter Things.

## Open the source-linked edition at Hello, World!

The located entry selects the whole *Hello, World!* chapter from the source-linked edition. Its entry Skene declares four reader operands (Chapter, Actions, ChapterAssets and Location). All four select the same Thing when opening this chapter. The Location for a whole chapter shares its chapter’s identity. A section has its own Location Thing.

Apply these files in the separate source-linked checkout prepared above. The resulting package requires `qn.tablet.reader` exactly 0.5.2.

```sh
book=courses/rust-book
cp "$book/editions/located/root.toml" "$book/Qrate.toml"
cp "$book/editions/located/entry.skene.pb" "$book/entry.skene.pb"
cp "$book/editions/located/entry.skene.textproto" "$book/entry.skene.textproto"
```

Publish and enroll the located collection for this checkout. The operator reviews the resulting package manifest and binds its entry to that collection. Quneic can then open the package from its registered root. The original course manifest continues to select the 0.5.1 reader in its own checkout.

To check the declared entry before enrollment, run the public verifier from the courseware repository. Set the paths to the compose descriptor, packaged 0.5.2 reader, released adapter, and protoc used for your build.

```sh
python3 courses/verify_entries.py --located --courses "$SOURCE_LINKED_CHECKOUT/courses" \
  --compose-schema "$COMPOSE_FDS" --reader-release "$READER_RELEASE" \
  --adapter "$ADAPTER" --protoc "$PROTOC"
```

## Publish the corrected Unsafe Rust reference

The **format 3** candidate uses `courseware.adapters` 0.5.1. In a separate checkout use
`editions/source-redirects.toml` in place of `editions/source-anchors.toml` in the
publication recipe above. Regenerate the connected root and map pins. The candidate
reads `book.toml`, the referring Markdown and the target chapter from the same CEL
commit. The published format 1 and 2 checkouts retain their own adapter, root, cuts
and exact entry selections.

In the pinned book, the `unsafe` reference in *Fixing Ownership Errors* names
`ch19-01-unsafe-rust.html`. The upstream redirect table maps it directly to
`ch20-01-unsafe-rust.html`. The corresponding Chapter is *Unsafe Rust* in the
pinned contents. Format 3 normalizes the destination of this Markdown reference
definition before publication. All other text, code, exercise IDs and source
file digests across the 120 Chapter records compare equal to format 2.
[Upstream source reference](https://github.com/cognitive-engineering-lab/rust-book/blob/88250e0392cef0622f318e35469108d68694c1f7/src/ch04-03-fixing-ownership-errors.md#L458) · [mdBook redirect](https://github.com/cognitive-engineering-lab/rust-book/blob/88250e0392cef0622f318e35469108d68694c1f7/book.toml#L50) · [Upstream Contents](https://github.com/cognitive-engineering-lab/rust-book/blob/88250e0392cef0622f318e35469108d68694c1f7/src/SUMMARY.md#L125)

The manifest change creates a new edition. The existing Learning Contents
release can derive its whole-chapter action using the ordinary target in
that edition. The existing Tablet reader presents the admitted action.
The operator must issue fresh root entries, section and figure grants,
and any learner effects for the successor cell identities before switching
a live checkout. Review saved learner positions separately. The standard
library reference still belongs to the separately published Rust library.

### Open format 3 with sections and figures

Use the format 3 course and regenerated connected root above. In the same
separate checkout install the matching package declaration and entry.

```sh
book=courses/rust-book
cp "$book/editions/located-redirects/root.toml" "$book/Qrate.toml"
cp "$book/editions/located-redirects/entry.skene.pb" "$book/entry.skene.pb"
cp "$book/editions/located-redirects/entry.skene.textproto" "$book/entry.skene.textproto"
```

Run the public `verify_entries.py --located` check shown above on these files.
The successor package is `courseware.rust-book` 0.5.2, uses adapter 0.5.1, and
requires reader 0.5.2. All four entry provisions select the format 3
*Hello, World!* Thing `c3b412a2-68dc-5fce-bdf0-458a0972eb9b`. The operator
enrolls independent source cuts for Chapter, Actions, ChapterAssets and
Location through the ordinary source-to-section publication recipe.

| Reviewed input | SHA-256 |
| --- | --- |
| Format 3 course manifest | `ceb61968aa5ae6fbb17232007db285d6d454589755dcc9214daac91d8d44d223` |
| Connected root generated with adapter 0.5.1 | `ecc010784fde91a374a377163fc033562f83f319ebaf96593a659ea16d9bf7a8` |
| Package manifest | `ec78209f092f03fa3b9b7b9772753ca957c0b26de6fe5685880a961621b98c21` |
| Entry Skene | `aaf16b1fd706048ccc54df3ba644199acd2e8be5196d1e3747399148e27e01e5` |
| Located reader Skene | `ffa40f124ed9537005a77458df17ba24c75fbc50ce4515ecd31ec39f395edc2c` |
| Located Contents Skene | `296f5e64afce227baec0de4f6c5fb8b4c4b216dfc26d5aa4da19e3dbc1a95ca2` |
| Section catalog | `1fcf0c1f0ae82e6ca420144553710ccf6851b72149137646bf40081ec29b7d3c` |
| Figure catalog | `cd0160cbd9b02232f2aa58bd76e6d772f9617d4c956d2a9109269158c11f125c` |

Verification opened every one of the 199 admitted members (120 chapters, 78
sections, one Contents hub) and found all 102 section-link occurrences in 47
chapters. The figure publisher admitted 22 figures. In one headless desktop
session the declared entry opened *Hello, World!*, the troubleshooting action
scrolled to the heading in *Installation*, the corrected action in *Fixing
Ownership Errors* opened *Unsafe Rust*, and the first *Introduction* figure
decoded to the pinned 1000 × 673 PNG. Learner position and exercise-effect
migration require independent enrollment for this edition.
