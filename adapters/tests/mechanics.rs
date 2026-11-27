use crate::{Block, Course};
use std::path::{Path, PathBuf};
use std::process::Command;

fn package() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../courses/mechanics-of-proof")
}

#[test]
fn official_contents_readings_and_exercise_share_a_single_git_revision() {
    let root = package();
    let course = Course::open(&root).unwrap();
    assert_eq!(course.tablet.id, "mechanics-of-proof");
    assert_eq!(course.revision, "e660f42b13ddcb6d12b52ba036d6bd071a0cfb9b");
    assert_eq!(course.workspace.language, "lean4");
    assert_eq!(course.navigation.roots.len(), 13);
    assert_eq!(course.chapters.len(), 60);
    let inventory = include_str!("../../courses/mechanics-of-proof/coverage.md");
    for chapter in &course.chapters {
        assert!(
            inventory.contains(&format!("| `{}` |", chapter.id)),
            "missing inventory row for {}",
            chapter.id
        );
        assert_eq!(chapter.revision, course.revision);
        assert!(chapter.blocks.iter().any(|b|matches!(b,Block::Markdown(s) if s.contains("https://hrmacbeth.github.io/math2001/"))));
    }
    let induction = course.chapter("06-induction--introduction").unwrap();
    assert!(induction.blocks.windows(2).any(|blocks| matches!(blocks, [Block::Markdown(text),Block::Code{language,source}] if text.contains("Qune courseware contributors") && language == "lean4" && source.contains("Nat.succ count = Nat.succ n") && source.trim_end().ends_with("sorry"))));
    let section = course.chapter("05-logic--logical-equivalence").unwrap();
    assert_eq!(section.title, "5.1. Logical equivalence");
    assert!(
        matches!(section.blocks.first(),Some(Block::Markdown(s)) if !s.contains("](https://") && s.split_whitespace().count() > 5)
    );
    let file = "Math2001/05_Logic/01_Logical_Equivalence.lean";
    let show = Command::new("git")
        .arg("-C")
        .arg(root.join("upstream"))
        .args(["show", &format!("{}:{file}", course.revision)])
        .output()
        .unwrap();
    assert!(show.status.success());
    let expected = String::from_utf8(show.stdout)
        .unwrap()
        .lines()
        .skip(82)
        .take(2)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert!(
        matches!(&section.blocks[4],Block::Code{language,source} if language=="lean4" && source==&expected)
    );
}
