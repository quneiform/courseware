use crate::{Block, Course};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

const GUIDE: &str = include_str!("../../courses/cs2023/topics.toml");
const DECLARATION: &str = include_str!("../../courses/cs2023/course.toml");

#[test]
fn loads_both_navigation_depths_offline_without_checks_or_git() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../courses/cs2023");
    let course =
        Course::open_with_git(&root, |_, _| panic!("link-only course invoked Git")).unwrap();
    assert_eq!(course.tablet.id, "cs2023");
    assert_eq!(course.workspace.editor, "reading");
    assert_eq!(course.workspace.default_chapter, "msf");
    let mut expected = vec![];
    for area in &course.navigation.roots {
        expected.push(area.chapter_id.as_str());
        expected.extend(area.children.iter().map(|child| child.chapter_id.as_str()));
    }
    assert_eq!(course.navigation.roots.len(), 17);
    assert_eq!(expected.len(), 17 + 166 + 4);
    for id in [
        "images-and-preimages",
        "induction-and-invariants",
        "vector-insertion",
        "sum-types-and-cases",
        "spd-sep-mobile",
    ] {
        assert!(expected.contains(&id));
    }
    let catalog: toml::Table = toml::from_str(GUIDE).unwrap();
    for area in catalog["areas"].as_array().unwrap() {
        let chapter = course
            .chapter(&area["id"].as_str().unwrap().to_lowercase())
            .unwrap();
        assert!(
            matches!(&chapter.blocks[0],Block::Markdown(body) if body.contains(area["source"].as_str().unwrap()))
        );
        for unit in area["units"].as_array().unwrap() {
            let id = unit["id"]
                .as_str()
                .unwrap()
                .to_lowercase()
                .replace('/', "-");
            let anchor = format!(
                "{}{}",
                catalog["report"].as_str().unwrap(),
                unit["anchor"].as_str().unwrap()
            );
            assert!(
                matches!(&course.chapter(&id).unwrap().blocks[0],Block::Markdown(body) if body.contains(&anchor))
            );
        }
    }
    assert_eq!(
        expected,
        course
            .chapters
            .iter()
            .map(|chapter| chapter.id.as_str())
            .collect::<Vec<_>>()
    );
    for chapter in &course.chapters {
        assert_eq!(chapter.revision, course.revision);
        let [Block::Markdown(text)] = chapter.blocks.as_slice() else {
            panic!("expected a link-only chapter")
        };
        assert!(text.contains("Joint Task Force") && text.contains("Qune courseware contributors"));
        assert!(course.question(&chapter.id).is_none());
        assert!(course.check(&chapter.id, &[]).is_err());
    }
    let topic = course.chapter("sum-types-and-cases").unwrap();
    let Block::Markdown(reading) = &topic.blocks[0] else {
        unreachable!()
    };
    assert!(
        reading.contains("FPL-Types, FPL-Functional")
            && reading.contains("Brown Cognitive Engineering Lab")
    );
}

#[test]
fn refuses_changed_edition_and_invalid_mappings() {
    let dir = std::env::temp_dir().join(format!("qune-cs2023-rejected-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let pin = format!("{:x}", Sha256::digest(GUIDE.as_bytes()));
    fs::write(dir.join("topics.toml"), GUIDE).unwrap();
    fs::write(
        dir.join("course.toml"),
        DECLARATION.replace(&pin, &"0".repeat(64)),
    )
    .unwrap();
    assert!(
        Course::open(&dir)
            .err()
            .unwrap()
            .to_string()
            .contains("SHA-256")
    );
    for (from, to) in [
        ("id = \"sum-types-and-cases\"", "id = \"msf\""),
        (
            "units = [\"AL-Foundational\"]\nreading",
            "units = [\"FPL-Types\"]\nreading",
        ),
        ("https://rust-book.cs.brown.edu/", "javascript:"),
        ("author = \"Heather Macbeth\"", "author = \"\""),
    ] {
        assert_ne!(GUIDE.replace(from, to), GUIDE);
        let invalid = GUIDE.replace(from, to);
        let digest = format!("{:x}", Sha256::digest(invalid.as_bytes()));
        fs::write(dir.join("topics.toml"), invalid).unwrap();
        fs::write(dir.join("course.toml"), DECLARATION.replace(&pin, &digest)).unwrap();
        assert!(Course::open(&dir).is_err(), "{from}");
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn shipped_placements_resolve_from_three_courses() {
    let packages = Path::new(env!("CARGO_MANIFEST_DIR")).join("../courses");
    let overlay = packages.join("connected-lessons");
    let cmd = Command::new(env!("CARGO_BIN_EXE_validate-learning-map"))
        .arg(overlay.join("root.json"))
        .arg(&packages)
        .arg(overlay.join("learning-map.toml"))
        .arg("--wire")
        .output()
        .unwrap();
    assert!(
        cmd.status.success(),
        "{}",
        String::from_utf8_lossy(&cmd.stderr)
    );
    let wire: serde_json::Value = serde_json::from_slice(&cmd.stdout).unwrap();
    let placements = wire["placements"].as_array().unwrap();
    assert_eq!(placements.len(), 3);
    assert!(
        placements
            .iter()
            .all(|p| p["exercise"] == placements[0]["exercise"])
    );
    assert_eq!(
        placements
            .iter()
            .map(|p| p["at"]["tabletRevision"].to_string())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
}
