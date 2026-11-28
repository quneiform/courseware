//! Session-local access to an already opened edition. Quiz keys stay in Course.
use crate::{
    Block, Course, NavigationNode, QuestionKind,
    mapping::{Catalog, MapStore},
};
use anyhow::{Context, Result, bail, ensure};
use buffa::Message;
use std::{
    io::{self, Read, Write},
    os::unix::ffi::OsStrExt,
    path::Path,
};

#[allow(dead_code, clippy::all)]
mod wire {
    include!(concat!(env!("OUT_DIR"), "/archive_wire.rs"));
}
use a::{answer::Value, query_request::Operation, request::Body as Req, response::Body as Res};
use wire::courseware::{archive::v1 as a, v05 as c};
const LIMIT: usize = 128 << 20;

fn read() -> Result<Option<a::Request>> {
    let mut r = io::stdin().lock();
    let mut len = [0; 4];
    if r.read(&mut len[..1])? == 0 {
        return Ok(None);
    }
    r.read_exact(&mut len[1..])?;
    let n = u32::from_be_bytes(len) as usize;
    ensure!(n > 0 && n <= LIMIT, "invalid request size");
    let mut data = vec![0; n];
    r.read_exact(&mut data)?;
    Ok(Some(a::Request::decode_from_slice(&data)?))
}
fn send(body: Res) -> Result<()> {
    let data = a::Response { body: Some(body) }.encode_to_vec();
    ensure!(data.len() <= LIMIT, "oversized response");
    let mut w = io::stdout().lock();
    w.write_all(&(data.len() as u32).to_be_bytes())?;
    w.write_all(&data)?;
    w.flush()?;
    Ok(())
}
fn git(dir: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let ["archive", "--format=tar", revision, "--", paths @ ..] = args else {
        bail!("expected pinned archive")
    };
    send(Res::Git(Box::new(a::GitArchive {
        directory: dir.as_os_str().as_bytes().into(),
        revision: (*revision).into(),
        paths: paths.iter().map(|p| p.to_string()).collect(),
    })))?;
    match read()?.and_then(|r| r.body) {
        Some(Req::Archive(bytes)) => Ok(bytes),
        _ => bail!("expected Git archive response"),
    }
}
fn nav(n: &NavigationNode) -> c::NavigationNode {
    c::NavigationNode {
        chapter_id: n.chapter_id.clone(),
        title: n.title.clone(),
        children: n.children.iter().map(nav).collect(),
    }
}
fn reading(course: &Course) -> c::CourseReading {
    let n = &course.navigation;
    let w = &course.workspace;
    c::CourseReading {
        revision: course.revision.clone(),
        tablet: Some(c::Tablet {
            id: course.tablet.id.clone(),
            title: course.tablet.title.clone(),
        })
        .into(),
        workspace: Some(c::Workspace {
            language: w.language.clone(),
            editor: w.editor.clone(),
            default_chapter: w.default_chapter.clone(),
        })
        .into(),
        navigation: Some(c::Navigation {
            id: n.id.clone(),
            title: n.title.clone(),
            kind: n.kind.clone(),
            source: n.source.clone(),
            roots: n.roots.iter().map(nav).collect(),
        })
        .into(),
        chapters: course
            .chapters
            .iter()
            .map(|ch| c::Chapter {
                id: ch.id.clone(),
                title: ch.title.clone(),
                revision: ch.revision.clone(),
                blocks: ch
                    .blocks
                    .iter()
                    .map(|b| {
                        use c::block::Content;
                        let content = match b {
                            Block::Markdown(s) => Content::Markdown(s.clone()),
                            Block::Notice(s) => Content::Notice(s.clone()),
                            Block::Interaction(id) => Content::InteractionId(id.clone()),
                            Block::Code { language, source } => Content::Code(Box::new(c::Code {
                                language: language.clone(),
                                source: source.clone(),
                            })),
                            Block::Steps { title, steps } => Content::Steps(Box::new(c::Steps {
                                title: title.clone(),
                                steps: steps
                                    .iter()
                                    .map(|(label, text)| c::Step {
                                        label: label.clone(),
                                        text: text.clone(),
                                    })
                                    .collect(),
                            })),
                        };
                        c::Block {
                            content: Some(content),
                        }
                    })
                    .collect(),
            })
            .collect(),
        questions: course
            .questions
            .values()
            .map(|entry| {
                let q = &entry.public;
                c::Question {
                    id: q.id.clone(),
                    kind: match q.kind {
                        QuestionKind::ShortAnswer => c::QuestionKind::SHORT_ANSWER,
                        QuestionKind::MultipleChoice => c::QuestionKind::MULTIPLE_CHOICE,
                        QuestionKind::Tracing => c::QuestionKind::TRACING,
                    }
                    .into(),
                    prompt: q.prompt.clone(),
                    starter: q.starter.clone(),
                    choices: q.choices.clone(),
                    multiple: q.multiple,
                    quiz: q.quiz.clone(),
                }
            })
            .collect(),
    }
}
enum Edition {
    One(Course),
    Connected(Catalog),
}
impl Edition {
    fn open(input: a::OpenRequest) -> Result<(Self, a::Opened)> {
        let dir = Path::new(std::ffi::OsStr::from_bytes(&input.directory)).canonicalize()?;
        let reader = if input.hosted_git {
            git
        } else {
            crate::source::native_git
        };
        if !input.catalog {
            let course = Course::open_with_git(&dir, reader)?;
            let result = a::Opened {
                courses: [(String::new(), reading(&course))].into_iter().collect(),
                ..Default::default()
            };
            return Ok((Self::One(course), result));
        }
        let cat = Catalog::open_with_git(
            &dir.join("root.json"),
            dir.parent().context("missing packages directory")?,
            reader,
        )?;
        let mut maps = MapStore::default();
        maps.reload(&cat, &dir.join("learning-map.toml"))?;
        let loc = |v: &crate::mapping::Address| a::Location {
            tablet: v.tablet.clone(),
            cell: v.cell.clone(),
        };
        let result = a::Opened {
            courses: cat
                .courses
                .iter()
                .map(|(alias, c)| (alias.clone(), reading(c)))
                .collect(),
            learning_root_json: serde_json::to_vec(&cat)?,
            placements: maps
                .current()
                .context("map not loaded")?
                .placements
                .iter()
                .map(|p| a::Placement {
                    id: p.id.clone(),
                    at: Some(loc(&p.at)).into(),
                    exercise: Some(loc(&p.exercise)).into(),
                    topic: p.topic.clone(),
                    assesses: p.assesses.clone().unwrap_or_default(),
                })
                .collect(),
        };
        Ok((Self::Connected(cat), result))
    }
    fn query(&self, q: a::QueryRequest) -> Result<a::Answer> {
        let course = || {
            match self {
                Self::One(c) if q.tablet.is_empty() => Some(c),
                Self::Connected(cat) => cat.courses.get(&q.tablet),
                _ => None,
            }
            .context("unknown course alias")
        };
        let blob =
            |media_type: String, data: Vec<u8>| Value::Blob(Box::new(a::Blob { media_type, data }));
        let value = match q.operation.as_known() {
            Some(Operation::CHECK) => Value::Correct(course()?.check(&q.key, &q.answers)?),
            Some(Operation::REVEAL) => Value::Explanation(course()?.reveal(&q.key)?),
            Some(Operation::ASSET) => {
                let (mime, data) = course()?.asset(&q.key)?;
                blob(mime, data)
            }
            Some(op @ (Operation::LESSON_MANIFEST | Operation::LESSON_ASSET)) => {
                let Self::Connected(cat) = self else {
                    bail!("connected catalog required")
                };
                let bytes = if op == Operation::LESSON_MANIFEST {
                    cat.lesson_manifest(&q.tablet)
                } else {
                    cat.lesson_asset(&q.tablet, &q.key)
                }
                .context("unknown pinned resource")?;
                blob(String::new(), bytes.to_vec())
            }
            _ => bail!("unknown query operation"),
        };
        Ok(a::Answer { value: Some(value) })
    }
}
pub(super) fn serve() -> Result<()> {
    let Some(Req::Open(open)) = read()?.and_then(|r| r.body) else {
        bail!("expected Open")
    };
    let (edition, reply) = match Edition::open(*open) {
        Ok(v) => v,
        Err(e) => return send(Res::Failure(e.to_string())),
    };
    send(Res::Opened(Box::new(reply)))?;
    while let Some(r) = read()? {
        let Some(Req::Query(q)) = r.body else {
            bail!("expected Query")
        };
        send(match edition.query(*q) {
            Ok(a) => Res::Answer(Box::new(a)),
            Err(e) => Res::Failure(e.to_string()),
        })?;
    }
    Ok(())
}
