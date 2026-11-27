//! Build the root's protobuf JSON from semantic names and locally installed editions.
use crate::mapping::{cell_ref, read_pin, wire};
pub(crate) fn course_pin(
    packages: &Path,
    repository: &str,
    manifest: &str,
) -> Result<wire::SourcePin> {
    Ok(read_pin(packages, repository, manifest)?.0)
}
pub(crate) fn pinned_cell(pin: &wire::SourcePin, name: &str) -> wire::CellRef {
    cell_ref(&pin.revision, name)
}
use crate::schema::quneic::learning::v1::{
    Exercise as WireExercise, Outcome as WireOutcome, Topic as WireTopic,
};
use crate::valid_identity;
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe {
    format_version: u32,
    id: String,
    courses: Vec<Source>,
    maps: Vec<Source>,
    topics: Vec<Topic>,
    outcomes: Vec<Outcome>,
    exercises: Vec<Exercise>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    alias: String,
    repository: String,
    manifest: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Topic {
    id: String,
    label: String,
    #[serde(default)]
    framework_uris: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Outcome {
    id: String,
    description: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Exercise {
    tablet: String,
    cell: String,
    outcome: String,
    checker: String,
    completion_rule: String,
}

pub fn build_root(packages: &Path, source: &str) -> Result<wire::LearningRoot> {
    let r: Recipe = toml::from_str(source)?;
    ensure!(
        r.format_version == 1 && valid_identity(&r.id),
        "invalid root recipe format or id"
    );
    let key = |kind: &str, name: &str| {
        wire::natural_id(&["quneic.learning.root-resource", &r.id, kind, name])
    };
    let mut aliases = BTreeMap::new();
    let mut add = |name: &str, id: &wire::Ref| -> Result<()> {
        ensure!(
            valid_identity(name) && aliases.insert(name.to_owned(), id.clone()).is_none(),
            "invalid or duplicate root alias {name}"
        );
        Ok(())
    };
    let mut pins = |sources: Vec<Source>| -> Result<Vec<wire::SourcePin>> {
        sources
            .into_iter()
            .map(|s| {
                let pin = course_pin(packages, &s.repository, &s.manifest)?;
                add(&s.alias, &pin.revision)?;
                Ok(pin)
            })
            .collect()
    };
    let courses = pins(r.courses)?;
    let maps = pins(r.maps)?;
    let mut topics = Vec::new();
    for t in r.topics {
        let id = key("topic", &t.id);
        add(&t.id, &id)?;
        topics.push(WireTopic {
            id: id.into(),
            label: Some(t.label),
            framework_uris: t.framework_uris,
        });
    }
    let mut outcomes = Vec::new();
    for o in r.outcomes {
        let id = key("outcome", &o.id);
        add(&o.id, &id)?;
        outcomes.push(WireOutcome {
            id: id.into(),
            description: Some(o.description),
        });
    }
    let mut exercises = Vec::new();
    for e in r.exercises {
        let tablet = aliases.get(&e.tablet).context("unknown exercise tablet")?;
        let outcome = aliases
            .get(&e.outcome)
            .filter(|o| outcomes.iter().any(|v| *v.id == **o))
            .context("unknown exercise outcome")?;
        let pin = courses
            .iter()
            .find(|p| &*p.revision == tablet)
            .context("exercise alias is not a course")?;
        ensure!(
            valid_identity(&e.checker) && valid_identity(&e.completion_rule),
            "invalid checker or completion rule"
        );
        exercises.push(WireExercise {
            cell: pinned_cell(pin, &e.cell).into(),
            outcome: outcome.clone().into(),
            checker: key("checker", &e.checker).into(),
            completion_rule: key("rule", &e.completion_rule).into(),
        });
    }
    Ok(wire::LearningRoot {
        id: key("root", &r.id).into(),
        courses,
        maps,
        topics,
        outcomes,
        exercises,
        aliases,
    })
}
