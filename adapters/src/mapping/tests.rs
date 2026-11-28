use super::{Catalog, MapStore, resolve};
use crate::authoring::build_root;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "courseware-map-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        let target = dir.join("overlay");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("s"), "starter").unwrap();
        fs::write(target.join("c"), "check").unwrap();
        let fixture = Self(dir);
        fixture.write_edition(&["one", "two", "exercise"]);
        fixture
    }
    fn write_edition(&self, cells: &[&str]) {
        let mut manifest = "format_version = 1\n".to_owned();
        for cell in cells {
            manifest += &format!(
                "[[lessons]]\nid={cell:?}\n[lessons.rust]\nstarter='s'\ncheck='c'\n[lessons.lean]\nstarter='s'\ncheck='c'\n"
            );
        }
        fs::write(self.0.join("overlay/lessons.toml"), manifest).unwrap();
    }
    fn path(&self) -> PathBuf {
        self.0.join("overlay/learning-map.toml")
    }
    fn pin(&self) -> Catalog {
        let recipe = r#"format_version=1
id="fixture"
courses=[{alias="book",repository="overlay",manifest="lessons.toml"}]
maps=[{alias="map",repository="overlay",manifest="learning-map.toml"}]
topics=[{id="sets",label="Sets"}]
outcomes=[{id="included",description="Prove inclusion"}]
exercises=[{tablet="book",cell="exercise",outcome="included",checker="check",completion_rule="both"}]
"#;
        let root = build_root(&self.0, recipe).unwrap();
        let path = self.0.join("root.json");
        fs::write(&path, serde_json::to_vec_pretty(&root).unwrap()).unwrap();
        Catalog::open(&path, &self.0).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
const MAP: &str = r#"format_version = 1
id = "test"
[[placements]]
id = "p1"
at = {tablet="book",cell="one"}
exercise = {tablet="book",cell="exercise"}
topic = "sets"
assesses = "included"
[[placements]]
id = "p2"
at = {tablet="book",cell="two"}
exercise = {tablet="book",cell="exercise"}
topic = "sets"
"#;

#[test]
fn two_cuts_roundtrip_located_rejections_and_last_good() {
    let f = Fixture::new();
    let path = f.path();
    fs::write(&path, MAP).unwrap();
    let catalog = f.pin();
    let mut store = MapStore::default();
    let first = store.reload(&catalog, &path).unwrap().clone();
    assert_eq!(
        first.wire.placements[0].exercise,
        first.wire.placements[1].exercise
    );
    assert_ne!(first.wire.placements[0].id, first.wire.placements[1].id);
    assert!(!first.wire.placements[1].assesses.is_set());
    let exp = first.export_toml().unwrap();
    let second = resolve(&catalog, &path, &exp).unwrap();
    assert_eq!(first.wire, second.wire);
    assert_eq!(exp, second.export_toml().unwrap());
    for (source, line) in [
        (MAP.replace("cell=\"one\"", "cell=\"absent\""), 5),
        (MAP.replace("\"p2\"", "\"p1\""), 10),
        (MAP.replace("topic = \"sets\"", "topic = \"typo\""), 7),
        (
            MAP.replace("assesses = \"included\"", "assesses = \"other\""),
            8,
        ),
        (MAP.replace("format_version = 1", "format_version = 2"), 1),
        (MAP.replace("topic =", "unknown ="), 7),
        (MAP.replace("cell=\"one\"", "cell=}"), 5),
    ] {
        fs::write(&path, source).unwrap();
        let e = store.reload(&catalog, &path).unwrap_err();
        assert_eq!(e.line, line, "{e}");
        assert!(e.column > 0);
        assert_eq!(e.path, path);
        assert_eq!(store.current().unwrap(), &first);
    }
    fs::write(&path, exp).unwrap();
    assert!(
        store
            .reload(&catalog, &path)
            .unwrap_err()
            .reason
            .contains("stale map")
    );
    let repinned = f.pin();
    let new_cut = store.reload(&repinned, &path).unwrap().cut.clone();
    assert_ne!(new_cut, first.cut);
    assert_eq!(store.at(&first.cut), Some(&first));
    assert_eq!(store.at(&new_cut).unwrap().wire, first.wire);
    assert_eq!(
        repinned.lesson_asset("book", "s"),
        Some("starter".as_bytes())
    );
    fs::write(f.0.join("overlay/s"), "drift").unwrap();
    assert_eq!(
        repinned.lesson_asset("book", "s"),
        Some("starter".as_bytes())
    );
    assert!(Catalog::open(&f.0.join("root.json"), &f.0).is_err());
    let files = fs::read_dir(&f.0).unwrap().count();
    assert_eq!(files, 2);
}
#[test]
fn repinned_edition_with_removed_passage_preserves_prior_cut() {
    let fixture = Fixture::new();
    let path = fixture.path();
    fs::write(&path, MAP).unwrap();
    let previous = fixture.pin();
    let mut store = MapStore::default();
    let accepted = store.reload(&previous, &path).unwrap().clone();

    fixture.write_edition(&["one", "exercise"]);
    let revised = fixture.pin();
    assert_ne!(
        previous.root.courses[0].revision,
        revised.root.courses[0].revision
    );
    assert_eq!(previous.root.maps, revised.root.maps);
    let error = store.reload(&revised, &path).unwrap_err();
    assert_eq!((error.path, error.line, error.column), (path, 11, 6));
    assert_eq!(error.reason, "unknown pinned cell book:two");
    assert_eq!(store.current(), Some(&accepted));
    assert_eq!(store.at(&accepted.cut), Some(&accepted));
}

#[test]
fn emitted_bytes_decode_with_pinned_types_and_identify_the_edit() {
    use crate::schema::{cel::expr, qune::core, quneic::learning::v1};
    use buffa::Message;
    let fixture = Fixture::new();
    fs::write(fixture.path(), MAP).unwrap();
    let catalog = fixture.pin();
    let resolved = resolve(&catalog, &fixture.path(), MAP).unwrap();
    let root = crate::export::material::root(&catalog);
    assert_eq!(root.diff.edits.len(), 7);
    let adapted = crate::export::adapted(&catalog, "book").unwrap();
    assert_eq!(adapted.exercises.len(), 1);
    assert_eq!(adapted.passages.len(), 3);
    let full = crate::export::material::course(&catalog, "book").unwrap();
    assert_eq!(full.diff.edits.len(), 16);
    assert!(crate::export::material::chapter(&catalog, "book", "exercise").is_ok());
    let encoded = resolved.wire.encode_to_vec();
    assert_eq!(
        v1::LearningMap::decode_from_slice(&encoded).unwrap(),
        resolved.wire
    );
    let posit = crate::export::posit(&resolved, &catalog.root.id);
    let decoded = core::Posit::decode_from_slice(&posit.encode_to_vec()).unwrap();
    let edit = &decoded.diff.edits[0];
    assert_eq!(
        edit.path.schema.descriptor.value.as_slice(),
        crate::export::schema_digest()
    );
    assert_eq!(edit.path.path.elements[0].field_number, Some(2));
    let Some(core::edit::Op::Value(value)) = &edit.op else {
        panic!("expected a literal")
    };
    let Some(expr::value::Kind::ListValue(list)) = &value.kind else {
        panic!("expected repeated placements")
    };
    for (item, expected) in list.values.iter().zip(&resolved.wire.placements) {
        let Some(expr::value::Kind::ObjectValue(any)) = &item.kind else {
            panic!("expected Placement")
        };
        assert_eq!(any.type_url, v1::Placement::TYPE_URL);
        assert_eq!(
            v1::Placement::decode_from_slice(&any.value).unwrap(),
            *expected
        );
    }
}

#[test]
fn passage_uses_pinned_cell_and_emits_ordered_tablet_fields() {
    use crate::{
        Block, Chapter, Course, Declaration,
        export::material,
        schema::{cel::expr::value::Kind, qune::core},
    };
    let fixture = Fixture::new();
    fs::write(fixture.path(), MAP).unwrap();
    let mut catalog = fixture.pin();
    let mut course = Course::empty(Declaration {
        format_version: 1,
        profile: "fixture".into(),
        revision: "r1".into(),
        tablet: Default::default(),
        navigation: Default::default(),
        workspace: Default::default(),
    });
    let blocks = vec![
        Block::Markdown("prose".into()),
        Block::Code {
            language: "rust".into(),
            source: "fn main() {}".into(),
        },
        Block::Interaction("quiz".into()),
        Block::Notice("notice".into()),
        Block::Steps {
            title: "walk".into(),
            steps: vec![("one".into(), "step".into())],
        },
    ];
    course.chapters.push(Chapter {
        id: "one".into(),
        title: "First".into(),
        revision: "r1".into(),
        blocks,
    });
    catalog.courses.insert("book".into(), course);
    let expected = super::cell_ref(&catalog.root.aliases["book"], "one").cell;
    let posit = material::chapter(&catalog, "book", "one").unwrap();
    assert_eq!(posit, material::chapter(&catalog, "book", "one").unwrap());
    assert_eq!(posit.diff.edits.len(), 4);
    for (i, edit) in posit.diff.edits.iter().enumerate() {
        assert!(
            matches!(&edit.target.pattern,Some(core::ref_pattern::Pattern::Thing(t)) if **t == *expected)
        );
        assert_eq!(edit.path.schema.full_name, "qn.tablet.v05.Chapter");
        assert_eq!(edit.path.schema.version.minor, 5);
        assert_eq!(edit.path.path.elements[0].field_number, Some(i as i32 + 1));
    }
    let Some(core::edit::Op::Value(value)) = &posit.diff.edits[3].op else {
        panic!("wanted chapter blocks")
    };
    let Some(Kind::ListValue(list)) = &value.kind else {
        panic!("wanted ordered chapter blocks")
    };
    let tags: Vec<u8> = list
        .values
        .iter()
        .map(|v| {
            let Some(Kind::ObjectValue(any)) = &v.kind else {
                panic!("wanted typed block")
            };
            assert_eq!(any.type_url, "type.googleapis.com/qn.tablet.v05.Block");
            any.value[0] >> 3
        })
        .collect();
    assert_eq!(tags, [1, 2, 3, 4, 5]);
    assert!(material::question(&catalog, "book", "absent").is_err());
    assert!(material::chapter(&catalog, "book", "absent").is_err());
}
