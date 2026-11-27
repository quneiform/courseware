//! A pinned, locally authored topic map with citations to the public CS2023 curriculum.
use crate::{Block, Chapter, Course, Declaration, NavigationNode};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Map {
    edition: String,
    curriculum_authors: String,
    curriculum: String,
    report: String,
    report_sha256: Option<String>,
    guide_author: String,
    areas: Vec<Area>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Area {
    id: String,
    title: String,
    source: String,
    #[serde(deserialize_with = "units")]
    units: Vec<Unit>,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    topics: Vec<Topic>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Unit {
    id: String,
    title: String,
    anchor: String,
}

// The first published map listed only unit codes. Its four authored chapters keep their old URLs.
fn units<'de, D: serde::Deserializer<'de>>(input: D) -> Result<Vec<Unit>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Entry {
        Citation(Unit),
        Code(String),
    }
    Ok(Vec::<Entry>::deserialize(input)?
        .into_iter()
        .map(|entry| match entry {
            Entry::Citation(unit) => unit,
            Entry::Code(id) => Unit {
                id,
                title: String::new(),
                anchor: String::new(),
            },
        })
        .collect())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Topic {
    id: String,
    title: String,
    units: Vec<String>,
    reading: String,
    references: Vec<Reference>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    author: String,
    title: String,
    url: String,
}

fn edition_url(url: &str, publisher: Option<&str>) -> bool {
    url.starts_with("https://")
        && url[8..].split_once('/').is_some_and(|(host, path)| {
            host.split('.').all(|part| {
                !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            }) && publisher.is_none_or(|expected| host == expected)
                && !path.is_empty()
                && !path
                    .chars()
                    .any(|c| c.is_whitespace() || c.is_control() || "()<>\\".contains(c))
        })
}

fn plain(s: &str) -> bool {
    !s.trim().is_empty() && !s.chars().any(|c| c.is_control() || "[]<>".contains(c))
}

impl Map {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.edition == "CS2023" && !self.areas.is_empty(),
            "expected CS2023 areas"
        );
        ensure!(
            plain(&self.curriculum_authors) && plain(&self.guide_author),
            "missing source or guide attribution"
        );
        ensure!(
            edition_url(&self.curriculum, Some("csed.acm.org"))
                && edition_url(&self.report, Some("csed.acm.org"))
                && self.report_sha256.as_ref().is_none_or(
                    |hash| hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit())
                ),
            "invalid curriculum citation"
        );
        let mut chapters = BTreeSet::new();
        for area in &self.areas {
            ensure!(
                !area.id.is_empty()
                    && area.id.bytes().all(|b| b.is_ascii_uppercase())
                    && chapters.insert(area.id.to_ascii_lowercase()),
                "invalid or duplicate knowledge area {}",
                area.id
            );
            ensure!(
                plain(&area.title)
                    && (area.summary.is_empty() || plain(&area.summary))
                    && edition_url(&area.source, Some("csed.acm.org")),
                "invalid area attribution or text for {}",
                area.id
            );
            let units: BTreeSet<_> = area.units.iter().map(|u| &u.id).collect();
            let prefix = format!("{}-", area.id);
            ensure!(
                !units.is_empty()
                    && units.len() == area.units.len()
                    && units
                        .iter()
                        .all(
                            |u| u.strip_prefix(&prefix).is_some_and(|name| !name.is_empty()
                                && name.split('/').all(|piece| !piece.is_empty()
                                    && piece
                                        .bytes()
                                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')))
                        ),
                "invalid knowledge units for {}",
                area.id
            );
            for unit in &area.units {
                if self.report_sha256.is_none() && unit.title.is_empty() && unit.anchor.is_empty() {
                    continue;
                }
                ensure!(
                    plain(&unit.title)
                        && unit
                            .anchor
                            .strip_prefix("#_Toc")
                            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
                        && chapters.insert(unit.id.to_lowercase().replace('/', "-")),
                    "invalid or duplicate unit {}",
                    unit.id
                );
            }
            for topic in &area.topics {
                ensure!(
                    !topic.id.is_empty()
                        && topic
                            .id
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                        && chapters.insert(topic.id.clone()),
                    "invalid or duplicate chapter {}",
                    topic.id
                );
                ensure!(
                    plain(&topic.title) && plain(&topic.reading),
                    "missing topic text for {}",
                    topic.id
                );
                ensure!(
                    !topic.units.is_empty()
                        && topic.units.iter().collect::<BTreeSet<_>>().len() == topic.units.len()
                        && topic.units.iter().all(|u| units.contains(u)),
                    "unresolved or duplicate knowledge unit for {}",
                    topic.id
                );
                for reference in &topic.references {
                    ensure!(
                        plain(&reference.title)
                            && plain(&reference.author)
                            && edition_url(&reference.url, None),
                        "invalid reading attribution for {}",
                        topic.id
                    );
                }
            }
        }
        Ok(())
    }
}

pub(super) fn open(
    root: &Path,
    spec: Declaration,
    _run_git: impl FnOnce(&Path, &[&str]) -> Result<Vec<u8>>,
) -> Result<Course> {
    ensure!(
        spec.workspace.editor == "reading",
        "CS2023 requires a reading workspace"
    );
    ensure!(
        crate::valid_identity(&spec.navigation.id)
            && plain(&spec.navigation.title)
            && spec.navigation.kind == "qn.ux.v05.TreeNavigation",
        "invalid CS2023 navigation declaration"
    );
    ensure!(
        spec.navigation.source == "topics.toml",
        "CS2023 navigation expects topics.toml"
    );
    let path = root.join("topics.toml");
    ensure!(
        fs::symlink_metadata(&path)?.is_file(),
        "topics.toml must be a regular course file"
    );
    let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    ensure!(
        format!("{:x}", Sha256::digest(&bytes)) == spec.revision,
        "CS2023 topics.toml does not match the declared SHA-256"
    );
    let map: Map =
        toml::from_str(std::str::from_utf8(&bytes)?).context("invalid CS2023 topics.toml")?;
    map.validate()?;
    let mut course = Course::empty(spec);
    let mut chapters = Vec::new();
    let mut roots = Vec::new();
    let mut add = |id: &str, title: &str, text: String| {
        chapters.push(Chapter {
            id: id.into(),
            title: title.into(),
            revision: course.revision.clone(),
            blocks: vec![Block::Markdown(text)],
        });
        NavigationNode {
            chapter_id: id.into(),
            title: title.into(),
            children: Vec::new(),
        }
    };
    for area in map.areas {
        let credit = format!(
            "Curriculum: {}. [CS2023 knowledge areas]({}) · [Final report]({})\n\nStudy guide: {}.",
            map.curriculum_authors, map.curriculum, map.report, map.guide_author
        );
        let source = format!("[{} ({})]({})", area.title, area.id, area.source);
        let mut branch = add(
            &area.id.to_ascii_lowercase(),
            &area.title,
            format!(
                "{}\n\nOfficial area: {}.\n\n{}",
                area.summary, source, credit
            ),
        );
        for unit in area.units.into_iter().filter(|u| !u.anchor.is_empty()) {
            let chapter_id = unit.id.to_ascii_lowercase().replace('/', "-");
            let reading = format!(
                "[{} · {}]({}{}) in the official curriculum.\n\n{}",
                unit.id, unit.title, map.report, unit.anchor, credit
            );
            branch.children.push(add(
                &chapter_id,
                &format!("{} · {}", unit.id, unit.title),
                reading,
            ));
        }
        for topic in area.topics {
            let mut text = format!(
                "{}\n\nCS2023 mapping: {} · {}.\n\n{}",
                topic.reading,
                source,
                topic.units.join(", "),
                credit
            );
            for reference in topic.references {
                text.push_str(&format!(
                    "\n\n[{}]({}) · {}",
                    reference.title, reference.url, reference.author
                ));
            }
            branch.children.push(add(&topic.id, &topic.title, text));
        }
        roots.push(branch);
    }
    course.chapters = chapters;
    course.navigation.roots = roots;
    Ok(course)
}
