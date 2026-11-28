//! Binary producer for independently compiled consumers of the owner contract.
#[path = "material.rs"]
pub(crate) mod material;
use crate::mapping::{Catalog, ResolvedMap, wire};
use crate::schema::{buf::validate, cel::expr, qune::core, quneic::learning::v1 as learning};
use anyhow::{Context, Result};
use buffa::Message;
use sha2::{Digest, Sha256};

impl Catalog {
    pub(crate) fn readings(&self, alias: &str) -> Option<&[crate::Chapter]> {
        self.courses
            .get(alias)
            .map(|c| c.chapters.as_slice())
            .or_else(|| self.lesson_data.get(alias).map(|d| d.chapters.as_slice()))
    }
}

pub(crate) fn adapted(catalog: &Catalog, alias: &str) -> Result<learning::AdaptedCourse> {
    let revision = catalog
        .root
        .aliases
        .get(alias)
        .context("unknown course alias")?;
    let pin = catalog
        .root
        .courses
        .iter()
        .find(|p| *p.revision == *revision)
        .context("alias is not a course")?;
    let passages = catalog
        .readings(alias)
        .into_iter()
        .flatten()
        .map(|chapter| learning::AdaptedPassage {
            passage: crate::authoring::pinned_cell(pin, &chapter.id).into(),
            origins: catalog
                .courses
                .get(alias)
                .and_then(|course| course.origins.get(&chapter.id))
                .map(|(path, digest)| learning::SourceAnchor {
                    path: Some(path.clone()),
                    file_digest: core::Digest {
                        algorithm: core::digest::Algorithm::Sha256.into(),
                        value: digest.clone(),
                    }
                    .into(),
                    ..Default::default()
                })
                .into_iter()
                .collect(),
        })
        .collect();
    let mut exercises: Vec<_> = catalog
        .root
        .exercises
        .iter()
        .filter(|e| *e.cell.tablet_revision == *revision)
        .cloned()
        .collect();
    if let Some(course) = catalog.courses.get(alias) {
        for id in course.questions.keys() {
            let cell = crate::authoring::pinned_cell(pin, id);
            if !exercises
                .iter()
                .any(|exercise| exercise.cell.as_option() == Some(&cell))
            {
                exercises.push(learning::Exercise {
                    cell: cell.into(),
                    ..Default::default()
                });
            }
        }
    }
    Ok(learning::AdaptedCourse {
        basis: learning::AdaptCourseRequest {
            source: pin.clone().into(),
            adapter: wire::natural_id(&["courseware.adapters", env!("CARGO_PKG_VERSION")]).into(),
            adaptation_rules: pin.clone().into(),
        }
        .into(),
        tablet_revision: revision.clone().into(),
        passages,
        exercises,
    })
}

pub(crate) fn posit(map: &ResolvedMap, source: &wire::Ref) -> core::Posit {
    let schema = core::SchemaRef {
        full_name: "quneic.learning.v1.LearningMap".into(),
        descriptor: core::Digest {
            algorithm: core::digest::Algorithm::Sha256.into(),
            value: schema_digest().to_vec(),
        }
        .into(),
        ..Default::default()
    };
    let objects = map
        .wire
        .placements
        .iter()
        .map(|placement| expr::Value {
            kind: Some(expr::value::Kind::ObjectValue(Box::new(
                buffa_types::google::protobuf::Any {
                    type_url: learning::Placement::TYPE_URL.into(),
                    value: placement.encode_to_vec().into(),
                    ..Default::default()
                },
            ))),
        })
        .collect();
    let path = validate::FieldPath {
        elements: vec![validate::FieldPathElement {
            field_number: Some(2),
            ..Default::default()
        }],
    };
    let edit = core::Edit {
        target: core::RefPattern {
            pattern: Some(core::ref_pattern::Pattern::Thing(Box::new(
                (*map.wire.id).clone(),
            ))),
        }
        .into(),
        path: core::FieldPath {
            schema: schema.into(),
            path: path.into(),
        }
        .into(),
        op: Some(core::edit::Op::Value(Box::new(expr::Value {
            kind: Some(expr::value::Kind::ListValue(Box::new(expr::ListValue {
                values: objects,
            }))),
        }))),
    };
    let identity = wire::natural_id(&[
        "quneic.learning.map-posit",
        &map.cut,
        &format!("{:x}", Sha256::digest(source.encode_to_vec())),
    ]);
    core::Posit {
        id: identity.uuid,
        attribution: Some(core::posit::Attribution::Source(Box::new(source.clone()))),
        diff: core::Diff { edits: vec![edit] }.into(),
    }
}

pub(crate) fn schema_digest() -> &'static [u8; 32] {
    include_bytes!(concat!(env!("OUT_DIR"), "/schema-digest"))
}
