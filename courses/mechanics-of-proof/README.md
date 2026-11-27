# The Mechanics of Proof

This Tablet follows Heather Macbeth’s Math 2001 book at Fordham University. Its navigation comes from the published Sphinx index at the pinned revision. The reader displays up to two opening paragraphs for each of its 60 entries, then links to the full section, which includes the original typeset mathematics, diagrams and worked solutions. The 13 top-level entries comprise a preface, ten chapters and two appendices. [Author’s edition](https://hrmacbeth.github.io/math2001/) · [Pinned source](https://github.com/hrmacbeth/math2001/tree/e660f42b13ddcb6d12b52ba036d6bd071a0cfb9b)

The text and Lean course files carry Heather Macbeth’s all-rights-reserved notices. The courseware repository records the upstream Git reference. The adapter reads the learner’s initialized checkout at runtime. Preserve the upstream notices and obtain the author’s permission before redistributing the book or adapted extracts.

## Work the first proof

Open **5. Logic → 5.1. Logical equivalence** and select the Lean listing after the reading excerpt. The starter is the first unfinished exercise in the official section file, identified by the revision, source path and line range in [exercise.toml](exercise.toml). It asks you to derive a disjunction from a conjunction. Replace the unfinished proof with your own proof. [Upstream statement](https://github.com/hrmacbeth/math2001/blob/e660f42b13ddcb6d12b52ba036d6bd071a0cfb9b/Math2001/05_Logic/01_Logical_Equivalence.lean#L83-L84)

## Verify the pinned exercise

From this directory, initialize the upstream checkout and install the compiler named in its toolchain file. The script needs Python 3.11 or later and elan.

```sh
git submodule update --init upstream
elan toolchain install leanprover/lean4:v4.3.0
python3 checks/validate.py
```

The check reads the statement from the pinned Git object, verifies the compiler identifier and checks three fixed proof bodies. Lean accepts the constructed disjunction, rejects a term of the wrong type, and rejects the original incomplete proof with warnings treated as errors. This example uses Lean core alone. The full upstream exercises use additional dependencies declared in the upstream Lake manifest.

## Add practice at another section

The section inventory lists all 60 stable cell IDs. In §8.3 the placement `macbeth-functions-to-images` opens the connected lesson on images and preimages. The public learning map records the passage, exercise and assessment separately. In §6.1 the editable **A loop counter invariant** listing gives the learner a smaller induction step to work on. Its author, prompt and Lean source live in the optional `[author_starter]` table. This listing is open practice. [Section inventory](coverage.md) · [Worked placement](../connected-lessons/learning-map.toml) · [Editable starter](exercise.toml)
