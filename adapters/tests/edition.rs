use courseware_adapters::{Block, Course, QuestionKind};
use std::{collections::BTreeSet, path::Path};

fn course() -> Course {
    Course::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../courses/rust-book")).unwrap()
}

#[test]
fn whole_pinned_course_imports_and_resolves_all_questions() {
    let book = course();
    let again = course();
    assert_eq!(book.chapters.len(), 120);
    assert_eq!(book.tablet.id, "rust-book");
    assert_eq!(book.tablet.title, "Rust Book");
    assert_eq!(book.navigation.id, "contents");
    assert_eq!(book.navigation.title, "Contents");
    assert_eq!(book.navigation.kind, "qn.ux.v05.TreeNavigation");
    assert_eq!(book.navigation.source, "src/SUMMARY.md");
    assert_eq!(book.navigation.roots.len(), 27);
    let first_part = &book.navigation.roots[4];
    assert_eq!(first_part.chapter_id, "ch01-00-getting-started");
    assert_eq!(first_part.children[1].chapter_id, "ch01-02-hello-world");
    assert!(
        book.navigation
            .chapters()
            .map(|n| (&n.chapter_id, &n.title))
            .eq(book.chapters.iter().map(|c| (&c.id, &c.title)))
    );
    assert_eq!(book.chapters, again.chapters);
    let mut quiz_files = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut kinds = [0; 3];
    let mut permission_examples = 0;
    let mut unavailable_examples = 0;
    for ch in &book.chapters {
        assert_eq!(ch.revision, book.revision);
        for b in &ch.blocks {
            match b {
                Block::Steps { title, steps } if title == "Permissions by line" => {
                    assert!(!steps.is_empty());
                    permission_examples += 1;
                }
                Block::Notice(message) if message.starts_with("Aquascope") => {
                    unavailable_examples += 1;
                }
                Block::Interaction(id) => {
                    assert!(ids.insert(id));
                    let question = book.question(id).unwrap();
                    quiz_files.insert(question.quiz.clone());
                    kinds[match question.kind {
                        QuestionKind::MultipleChoice => 0,
                        QuestionKind::Tracing => 1,
                        QuestionKind::ShortAnswer => 2,
                    }] += 1;
                }
                Block::Markdown(s) | Block::Code { source: s, .. } => assert!(
                    !s.contains("{{#quiz ")
                        && !s.contains("<span class=\"caption\">")
                        && !s.contains("<span class=\"filename\">")
                        && !s.contains("{{#include ")
                        && !s.contains("{{#rustdoc_include "),
                    "{}",
                    ch.id
                ),
                _ => {}
            }
        }
    }
    assert_eq!((permission_examples, unavailable_examples), (39, 42));
    assert_eq!(quiz_files.len(), 93);
    assert_eq!(kinds, [141, 64, 17]);
    let hello = book.chapter("ch01-02-hello-world").unwrap();
    assert!(hello.blocks.iter().any(|b|matches!(b,Block::Code {source,..} if source.contains("fn main()") && source.contains("Hello, world!"))));
    let (mime, img) = book.asset("img/trpl15-01.svg").unwrap();
    assert_eq!(mime, "image/svg+xml");
    assert!(!img.is_empty());
    assert!(book.asset("../quizzes/ch01-02-hello-world.toml").is_err());
    let borrowing = book.chapter("ch04-02-references-and-borrowing").unwrap();
    assert!(borrowing.blocks.windows(2).any(|p|matches!(p,[Block::Code{source,..},Block::Steps{steps,..}] if source.contains("v.push(4)")&&steps.len()==4&&steps[1].1.contains("loses write and own"))));
}

#[test]
fn grade_real_choice_short_answer_and_tracing_responses() {
    let book = course();
    let short = "87c3b09e-84d3-49d7-8680-0b2d068a7fc5";
    assert!(!book.check(short, &["start".into()]).unwrap());
    assert!(book.check(short, &[" Main ".into()]).unwrap());
    assert!(book.reveal(short).unwrap().contains("fn main()"));
    let choice = "0d78d302-eb3d-4ac7-a1ac-f2d661810744";
    let choices = &book.question(choice).unwrap().choices;
    let correct = choices
        .iter()
        .find(|c| c.contains("generates a binary executable"))
        .unwrap();
    assert!(book.check(choice, std::slice::from_ref(correct)).unwrap());
    let compile = "8ef2f170-71d9-4f17-a688-b99d8c8072a8";
    assert!(
        book.check(compile, &["compiles".into(), "  c2\n".into()])
            .unwrap()
    );
    assert!(
        !book
            .check(compile, &["compiles".into(), "b2".into()])
            .unwrap()
    );
    let error = "153b12b3-4fbc-449c-99a6-7a2cd469df0f";
    assert!(book.check(error, &["error".into()]).unwrap());
    assert!(!book.check(error, &["compiles".into(), "".into()]).unwrap());
    assert!(book.check("unknown", &[]).is_err());
}

#[test]
fn grading_matches_every_authored_key_in_the_pinned_repository() {
    let book = course();
    let upstream = Path::new(env!("CARGO_MANIFEST_DIR")).join("../courses/rust-book/upstream");
    let mut files = BTreeSet::new();
    for ch in &book.chapters {
        for block in &ch.blocks {
            if let Block::Interaction(id) = block {
                files.insert(&book.question(id).unwrap().quiz);
            }
        }
    }
    let mut count = 0;
    for path in files {
        let text = std::fs::read_to_string(upstream.join(path)).unwrap();
        let quiz: toml::Table = toml::from_str(&text).unwrap();
        for q in quiz["questions"].as_array().unwrap() {
            let id = q["id"].as_str().unwrap();
            let key = &q["answer"];
            let responses = match q["type"].as_str().unwrap() {
                "MultipleChoice" => {
                    let choices = &key["answer"];
                    let expected = if let Some(value) = choices.as_str() {
                        vec![value.to_owned()]
                    } else {
                        choices
                            .as_array()
                            .unwrap()
                            .iter()
                            .rev()
                            .map(|v| v.as_str().unwrap().to_owned())
                            .collect()
                    };
                    let distractors = q["prompt"]["distractors"].as_array().unwrap();
                    for distractor in distractors {
                        assert!(
                            !book
                                .check(id, &[distractor.as_str().unwrap().into()])
                                .unwrap(),
                            "{id}"
                        );
                    }
                    vec![expected]
                }
                "ShortAnswer" => {
                    let mut alternatives = vec![vec![key["answer"].as_str().unwrap().into()]];
                    if let Some(values) = key.get("alternatives") {
                        for value in values.as_array().unwrap() {
                            alternatives.push(vec![value.as_str().unwrap().into()]);
                        }
                    }
                    alternatives
                }
                "Tracing" if key["doesCompile"].as_bool().unwrap() => {
                    assert!(!book.check(id, &["error".into()]).unwrap(), "{id}");
                    vec![vec![
                        "compiles".into(),
                        key["stdout"].as_str().unwrap().into(),
                    ]]
                }
                "Tracing" => {
                    assert!(
                        !book.check(id, &["compiles".into(), String::new()]).unwrap(),
                        "{id}"
                    );
                    vec![vec!["error".into()]]
                }
                _ => panic!("unknown authored question type"),
            };
            for response in responses {
                assert!(book.check(id, &response).unwrap(), "{id}");
            }
            assert!(!book.check(id, &[]).unwrap(), "{id}");
            count += 1;
        }
    }
    assert_eq!(count, 222);
}

#[test]
fn broken_listing_reports_its_source_line_and_preserves_the_open_edition() {
    let original = course();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../courses/rust-book");
    let result = Course::open_with_git(&root, |_, _| {
        let mut archive = tar::Builder::new(Vec::new());
        for (path, source) in [
            ("src/SUMMARY.md", "- [Broken](broken.md)"),
            (
                "src/broken.md",
                "# Sample\n```rust\n{{#include missing.rs}}\n```",
            ),
        ] {
            let mut entry = tar::Header::new_gnu();
            entry.set_size(source.len() as u64);
            entry.set_mode(0o644);
            entry.set_cksum();
            archive.append_data(&mut entry, path, source.as_bytes())?;
        }
        Ok(archive.into_inner()?)
    });
    let error = format!(
        "{:#}",
        result.err().expect("missing listing should refuse import")
    );
    assert!(
        error.contains("src/broken.md:3") && error.contains("missing.rs"),
        "{error}"
    );
    assert_eq!(
        original.chapter("ch01-02-hello-world").unwrap().title,
        "Hello, World!"
    );
}
