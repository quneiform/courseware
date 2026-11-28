//! Encode one pinned passage or public quiz prompt at a time.
use super::{Catalog, core, expr, validate, wire};
use crate::{Block, QuestionKind};
use anyhow::{Context, Result};
use buffa::{Message, MessageName};
use sha2::{Digest, Sha256};

#[allow(dead_code, clippy::all)]
mod owner {
    include!(concat!(env!("OUT_DIR"), "/tablet.rs"));
    include!(concat!(env!("OUT_DIR"), "/courseware_types.rs"));
}
use owner::{courseware::v05 as questions, qn::tablet::v05 as tablet};

fn block(b: &Block) -> tablet::Block {
    use tablet::block::Content;
    let content = match b {
        Block::Markdown(s) => Content::Markdown(s.clone()),
        Block::Code { language, source } => Content::Code(Box::new(tablet::Code {
            language: language.clone(),
            source: source.clone(),
        })),
        Block::Notice(s) => Content::Notice(s.clone()),
        Block::Interaction(id) => Content::InteractionId(id.clone()),
        Block::Steps { title, steps } => Content::Sequence(Box::new(tablet::Steps {
            title: title.clone(),
            steps: steps
                .iter()
                .map(|(label, markdown)| tablet::Step {
                    label: label.clone(),
                    markdown: markdown.clone(),
                })
                .collect(),
        })),
    };
    tablet::Block {
        content: Some(content),
    }
}

pub(crate) fn chapter(catalog: &Catalog, alias: &str, id: &str) -> Result<core::Posit> {
    let revision = catalog
        .root
        .aliases
        .get(alias)
        .context("unknown course alias")?;
    let passage = catalog
        .readings(alias)
        .context("alias has no reading")?
        .iter()
        .find(|p| p.id == id)
        .context("unknown course passage")?;
    let target = crate::mapping::cell_ref(revision, id).cell;
    Ok(make_posit(
        catalog,
        "chapter-posit",
        chapter_edits(&target, passage),
    ))
}

fn object<T: Message + MessageName>(t: &T) -> expr::value::Kind {
    expr::value::Kind::ObjectValue(Box::new(buffa_types::google::protobuf::Any {
        type_url: T::TYPE_URL.into(),
        value: t.encode_to_vec().into(),
        ..Default::default()
    }))
}

fn list(kinds: impl Iterator<Item = expr::value::Kind>) -> expr::value::Kind {
    expr::value::Kind::ListValue(Box::new(expr::ListValue {
        values: kinds.map(|kind| expr::Value { kind: Some(kind) }).collect(),
    }))
}

fn learning_schema<T: MessageName>() -> core::SchemaRef {
    core::SchemaRef {
        full_name: T::FULL_NAME.into(),
        descriptor: core::Digest {
            algorithm: core::digest::Algorithm::Sha256.into(),
            value: super::schema_digest().to_vec(),
        }
        .into(),
        ..Default::default()
    }
}
pub(crate) fn root(catalog: &Catalog) -> core::Posit {
    let root = &catalog.root;
    let entries = root
        .aliases
        .iter()
        .map(|(alias, thing)| expr::map_value::Entry {
            key: expr::Value {
                kind: Some(expr::value::Kind::StringValue(alias.clone())),
            }
            .into(),
            value: expr::Value {
                kind: Some(object(thing)),
            }
            .into(),
        })
        .collect();
    let fields = [
        object(&*root.id),
        list(root.courses.iter().map(object)),
        list(root.maps.iter().map(object)),
        list(root.topics.iter().map(object)),
        list(root.outcomes.iter().map(object)),
        list(root.exercises.iter().map(object)),
        expr::value::Kind::MapValue(Box::new(expr::MapValue { entries })),
    ];
    let edits = field_edits(
        &root.id,
        learning_schema::<super::learning::LearningRoot>(),
        fields,
    );
    make_posit(catalog, "root-posit", edits)
}

pub(crate) fn course(catalog: &Catalog, alias: &str) -> Result<core::Posit> {
    let index = super::adapted(catalog, alias)?;
    let target = &*index.tablet_revision;
    let fields = [
        object(&*index.basis),
        object(target),
        list(index.passages.iter().map(object)),
        list(index.exercises.iter().map(object)),
    ];
    let schema = learning_schema::<super::learning::AdaptedCourse>();
    let mut edits = field_edits(target, schema, fields);
    if let Some(chapters) = catalog.readings(alias) {
        anyhow::ensure!(
            index.passages.len() == chapters.len(),
            "course index and reading count differ"
        );
        for (record, passage) in index.passages.iter().zip(chapters) {
            edits.extend(chapter_edits(&record.passage.cell, passage));
        }
    }
    Ok(make_posit(catalog, "course-posit", edits))
}

fn chapter_edits(target: &wire::Ref, passage: &crate::Chapter) -> Vec<core::Edit> {
    use expr::value::Kind;
    let fields = [
        Kind::StringValue(passage.id.clone()),
        Kind::StringValue(passage.title.clone()),
        Kind::StringValue(passage.revision.clone()),
        list(passage.blocks.iter().map(|b| object(&block(b)))),
    ];
    let schema = core::SchemaRef {
        full_name: "qn.tablet.v05.Chapter".into(),
        version: core::SemVer {
            minor: 5,
            ..Default::default()
        }
        .into(),
        descriptor: core::Digest {
            algorithm: core::digest::Algorithm::Sha256.into(),
            value: include_bytes!(concat!(env!("OUT_DIR"), "/tablet-digest")).to_vec(),
        }
        .into(),
    };
    field_edits(target, schema, fields)
}

fn field_edits<const N: usize>(
    target: &wire::Ref,
    schema: core::SchemaRef,
    fields: [expr::value::Kind; N],
) -> Vec<core::Edit> {
    fields
        .into_iter()
        .enumerate()
        .map(|(index, kind)| core::Edit {
            target: core::RefPattern {
                pattern: Some(core::ref_pattern::Pattern::Thing(Box::new(
                    (*target).clone(),
                ))),
            }
            .into(),
            path: core::FieldPath {
                schema: schema.clone().into(),
                path: validate::FieldPath {
                    elements: vec![validate::FieldPathElement {
                        field_number: Some(index as i32 + 1),
                        ..Default::default()
                    }],
                }
                .into(),
            }
            .into(),
            op: Some(core::edit::Op::Value(Box::new(expr::Value {
                kind: Some(kind),
            }))),
        })
        .collect()
}

fn make_posit(catalog: &Catalog, mode: &str, edits: Vec<core::Edit>) -> core::Posit {
    let diff = core::Diff { edits };
    let identity = wire::natural_id(&[
        &format!("courseware.adapters/{mode}"),
        &format!("{:x}", Sha256::digest(diff.encode_to_vec())),
        &format!("{:x}", Sha256::digest(catalog.root.id.encode_to_vec())),
    ]);
    core::Posit {
        id: identity.uuid,
        attribution: Some(core::posit::Attribution::Source(Box::new(
            (*catalog.root.id).clone(),
        ))),
        diff: diff.into(),
    }
}

pub(crate) fn question(catalog: &Catalog, alias: &str, id: &str) -> Result<questions::Question> {
    let q = catalog
        .courses
        .get(alias)
        .context("alias has no reading")?
        .question(id)
        .context("unknown course question")?;
    let kind = match q.kind {
        QuestionKind::MultipleChoice => questions::QuestionKind::MULTIPLE_CHOICE,
        QuestionKind::ShortAnswer => questions::QuestionKind::SHORT_ANSWER,
        QuestionKind::Tracing => questions::QuestionKind::TRACING,
    };
    Ok(questions::Question {
        id: q.id.clone(),
        kind: kind.into(),
        prompt: q.prompt.clone(),
        starter: q.starter.clone(),
        choices: q.choices.clone(),
        multiple: q.multiple,
        quiz: q.quiz.clone(),
    })
}

pub(crate) fn question_cell(catalog: &Catalog, alias: &str, id: &str) -> Result<wire::CellRef> {
    question(catalog, alias, id)?;
    let revision = catalog
        .root
        .aliases
        .get(alias)
        .context("missing pinned course")?;
    Ok(crate::mapping::cell_ref(revision, id))
}

/// Check with the private key from the same pinned course that produced the public prompt.
pub(crate) fn check_short_answer(
    catalog: &Catalog,
    alias: &str,
    id: &str,
    public_sha256: &str,
    response: String,
) -> Result<buffa_types::google::protobuf::BoolValue> {
    let published = question(catalog, alias, id)?;
    anyhow::ensure!(
        published.kind == questions::QuestionKind::SHORT_ANSWER,
        "expected a short-answer question"
    );
    anyhow::ensure!(
        format!("{:x}", Sha256::digest(published.encode_to_vec())) == public_sha256,
        "published question digest differs"
    );
    let course = catalog
        .courses
        .get(alias)
        .context("missing pinned course")?;
    Ok(buffa_types::google::protobuf::BoolValue {
        value: course.check(id, &[response])?,
        ..Default::default()
    })
}
