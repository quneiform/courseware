use crate::{Block, Course, QuestionKind};
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

#[test]
fn selected_cel_prompts_decode_from_the_command_using_the_public_schema() {
    use buffa::Message;
    use std::process::Command;
    #[allow(dead_code, clippy::all)]
    mod consumer {
        include!(concat!(env!("OUT_DIR"), "/courseware_types.rs"));
    }
    use consumer::courseware::v05 as wire;

    let courses = Path::new(env!("CARGO_MANIFEST_DIR")).join("../courses");
    let root = courses.join("connected-lessons/root.json");
    let map = courses.join("connected-lessons/learning-map.toml");
    let book = course();
    for (id, kind) in [
        (
            "87c3b09e-84d3-49d7-8680-0b2d068a7fc5",
            wire::QuestionKind::SHORT_ANSWER,
        ),
        (
            "0d78d302-eb3d-4ac7-a1ac-f2d661810744",
            wire::QuestionKind::MULTIPLE_CHOICE,
        ),
        (
            "8ef2f170-71d9-4f17-a688-b99d8c8072a8",
            wire::QuestionKind::TRACING,
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_validate-learning-map"))
            .args([&root, &courses, &map])
            .args(["--question-wire", "rust-book", id])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let actual = wire::Question::decode_from_slice(&output.stdout).unwrap();
        let original = book.question(id).unwrap();
        assert_eq!(actual.id, id);
        assert_eq!(actual.kind, kind);
        assert_eq!(actual.prompt, original.prompt);
        assert_eq!(actual.choices, original.choices);
        assert_eq!(actual.starter, original.starter);
        assert_eq!(actual.quiz, original.quiz);
        assert_eq!(actual.multiple, original.multiple);
    }
}

#[test]
fn atomic_course_posits_cover_exactly_the_indexed_things_and_fields() {
    use buffa::Message;
    use learning_consumer::{qune::core, quneic::learning::v1};
    use std::process::Command;
    let courses = Path::new(env!("CARGO_MANIFEST_DIR")).join("../courses");
    for alias in [
        "rust-book",
        "mechanics-of-proof",
        "cs2023",
        "connected-lessons",
    ] {
        let export = |mode: &str| {
            let result = Command::new(env!("CARGO_BIN_EXE_validate-learning-map"))
                .arg(courses.join("connected-lessons/root.json"))
                .arg(&courses)
                .arg(courses.join("connected-lessons/learning-map.toml"))
                .args([mode, alias])
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            result.stdout
        };
        let encoded = export("--course-posit");
        assert_eq!(encoded, export("--course-posit"));
        let index = v1::AdaptedCourse::decode_from_slice(&export("--adapted-course")).unwrap();
        let mut expected = BTreeSet::new();
        for (thing, name) in
            std::iter::once((&*index.tablet_revision, "quneic.learning.v1.AdaptedCourse")).chain(
                index
                    .passages
                    .iter()
                    .map(|p| (&*p.passage.cell, "qn.tablet.v05.Chapter")),
            )
        {
            for field in 1..=4 {
                assert!(expected.insert((thing.encode_to_vec(), name.to_owned(), field)));
            }
        }
        let posit = core::Posit::decode_from_slice(&encoded).unwrap();
        for edit in posit.diff.edits.iter() {
            let Some(core::ref_pattern::Pattern::Thing(thing)) = &edit.target.pattern else {
                panic!("expected explicit thing")
            };
            let fields = &edit.path.path.elements;
            assert_eq!(fields.len(), 1);
            assert!(
                expected.remove(&(
                    thing.encode_to_vec(),
                    edit.path.schema.full_name.clone(),
                    fields[0].field_number.unwrap()
                )),
                "unexpected or duplicate field"
            );
        }
        assert!(expected.is_empty(), "missing fields for {alias}");
    }
}

#[allow(dead_code, clippy::all)]
mod learning_consumer {
    include!(concat!(env!("OUT_DIR"), "/schema.rs"));
}

#[test]
fn catalog_publication_matches_every_field_of_the_pinned_root() {
    use buffa::{Message, MessageName};
    use learning_consumer::{cel::expr::value::Kind, qune::core, quneic::learning::v1};
    use std::process::Command;
    let courses = Path::new(env!("CARGO_MANIFEST_DIR")).join("../courses");
    let export = |flag: &str| {
        let output = Command::new(env!("CARGO_BIN_EXE_validate-learning-map"))
            .arg(courses.join("connected-lessons/root.json"))
            .arg(&courses)
            .arg(courses.join("connected-lessons/learning-map.toml"))
            .arg(flag)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    };
    let bytes = export("--root-posit");
    assert_eq!(bytes, export("--root-posit"));
    let expected = v1::LearningRoot::decode_from_slice(&export("--root-wire")).unwrap();
    let publication = core::Posit::decode_from_slice(&bytes).unwrap();
    assert_eq!(publication.diff.edits.len(), 7);
    let fields=publication.diff.edits.iter().enumerate().map(|(i,e)|{
        assert!(matches!(&e.target.pattern,Some(core::ref_pattern::Pattern::Thing(target)) if **target==*expected.id));
        assert_eq!(e.path.schema.full_name,v1::LearningRoot::FULL_NAME);
        assert_eq!(e.path.path.elements.len(),1);
        assert_eq!(e.path.path.elements[0].field_number,Some(i as i32+1));
        let Some(core::edit::Op::Value(v))=&e.op else{panic!("expected literal")};
        v.kind.as_ref().unwrap()
    }).collect::<Vec<_>>();
    fn message<T: Message + MessageName>(k: &Kind) -> T {
        let Kind::ObjectValue(a) = k else {
            panic!("expected typed message")
        };
        assert_eq!(a.type_url, T::TYPE_URL);
        T::decode_from_slice(&a.value).unwrap()
    }
    fn messages<T: Message + MessageName>(k: &Kind) -> Vec<T> {
        let Kind::ListValue(l) = k else {
            panic!("expected repeated field")
        };
        l.values
            .iter()
            .map(|v| message(v.kind.as_ref().unwrap()))
            .collect()
    }
    assert_eq!(message::<core::ThingRef>(fields[0]), *expected.id);
    assert_eq!(messages::<v1::SourcePin>(fields[1]), expected.courses);
    assert_eq!(messages::<v1::SourcePin>(fields[2]), expected.maps);
    assert_eq!(messages::<v1::Topic>(fields[3]), expected.topics);
    assert_eq!(messages::<v1::Outcome>(fields[4]), expected.outcomes);
    assert_eq!(messages::<v1::Exercise>(fields[5]), expected.exercises);
    let Kind::MapValue(aliases) = fields[6] else {
        panic!("expected alias map")
    };
    assert_eq!(aliases.entries.len(), expected.aliases.len());
    for (entry, (key, target)) in aliases.entries.iter().zip(&expected.aliases) {
        assert!(matches!(&entry.key.kind,Some(Kind::StringValue(k)) if k==key));
        assert_eq!(
            message::<core::ThingRef>(entry.value.kind.as_ref().unwrap()),
            *target
        );
    }
}

#[test]
fn admitted_short_answer_uses_the_same_private_cel_key_at_the_exact_public_digest() {
    use buffa::Message;
    use buffa_types::google::protobuf::{BoolValue, StringValue};
    use sha2::{Digest, Sha256};
    use std::{
        io::Write,
        process::{Command, Stdio},
    };

    let courses = Path::new(env!("CARGO_MANIFEST_DIR")).join("../courses");
    let overlay = courses.join("connected-lessons");
    let run = |flags: &[&str], input: &[u8]| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_validate-learning-map"))
            .arg(overlay.join("root.json"))
            .arg(&courses)
            .arg(overlay.join("learning-map.toml"))
            .args(flags)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    };
    let id = "87c3b09e-84d3-49d7-8680-0b2d068a7fc5";
    let published = run(&["--question-wire", "rust-book", id], &[]);
    assert!(
        published.status.success(),
        "{}",
        String::from_utf8_lossy(&published.stderr)
    );
    let digest = format!("{:x}", Sha256::digest(&published.stdout));
    let cell = run(&["--question-cell", "rust-book", id], &[]);
    assert!(cell.status.success() && !cell.stdout.is_empty());
    let check = |answer: &str, pin: &str| {
        run(
            &["--check-question", "rust-book", id, pin],
            &StringValue {
                value: answer.into(),
                ..Default::default()
            }
            .encode_to_vec(),
        )
    };
    for (answer, correct) in [("  MAIN ", true), ("start", false)] {
        let result = check(answer, &digest);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            BoolValue::decode_from_slice(&result.stdout).unwrap().value,
            correct
        );
    }
    assert!(!check("main", &"0".repeat(64)).status.success());
    let choice = "0d78d302-eb3d-4ac7-a1ac-f2d661810744";
    let prompt = run(&["--question-wire", "rust-book", choice], &[]);
    assert!(prompt.status.success());
    let wrong_kind = run(
        &[
            "--check-question",
            "rust-book",
            choice,
            &format!("{:x}", Sha256::digest(prompt.stdout)),
        ],
        &[],
    );
    assert!(!wrong_kind.status.success());
}
