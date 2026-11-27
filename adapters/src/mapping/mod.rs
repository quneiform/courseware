//! Validate and retain authored maps over pinned, locally installed course packages.

#[cfg(test)]
mod tests;
pub(crate) mod wire;

use crate::{Course, valid_identity};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    path::{Component, Path, PathBuf},
};
use toml::Spanned;
use wire::{LearningMap, LearningRoot};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Address {
    pub tablet: String,
    pub cell: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredPlacement {
    pub id: String,
    pub at: Address,
    pub exercise: Address,
    pub topic: String,
    pub assesses: Option<String>,
}
#[derive(Serialize)]
struct Export<'a> {
    format_version: u32,
    id: &'a str,
    placements: &'a [AuthoredPlacement],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Authored {
    format_version: Spanned<u32>,
    id: Spanned<String>,
    placements: Vec<Spanned<InputPlacement>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputPlacement {
    id: Spanned<String>,
    at: Spanned<Address>,
    exercise: Spanned<Address>,
    topic: Spanned<String>,
    assesses: Option<Spanned<String>>,
}
#[derive(Debug)]
pub struct Diagnostic {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub reason: String,
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}",
            self.path.display(),
            self.line,
            self.column,
            self.reason
        )
    }
}
impl std::error::Error for Diagnostic {}
fn diagnostic(path: &Path, source: &str, offset: usize, reason: impl fmt::Display) -> Diagnostic {
    let prefix = &source[..offset.min(source.len())];
    Diagnostic {
        path: path.into(),
        line: prefix.bytes().filter(|b| *b == b'\n').count() + 1,
        column: prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1,
        reason: reason.to_string(),
    }
}

pub struct Catalog {
    pub courses: BTreeMap<String, Course>,
    pub(crate) root: LearningRoot,
    pub(crate) lesson_data: BTreeMap<String, LessonData>,
    pub(crate) cells: BTreeMap<String, BTreeSet<String>>,
    map_paths: BTreeMap<PathBuf, wire::SourcePin>,
}
// Serialize the exact admitted root. This keeps the transitional standalone reader on the
// same pinned input as its source catalog, without exposing consumer-side protobuf types.
impl Serialize for Catalog {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.root.serialize(serializer)
    }
}
fn digest(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
fn within(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    ensure!(
        !relative.is_empty() && path.components().all(|c| matches!(c, Component::Normal(_))),
        "invalid root-relative path {relative:?}"
    );
    let absolute = root
        .join(path)
        .canonicalize()
        .with_context(|| format!("open {relative}"))?;
    ensure!(
        absolute.starts_with(root),
        "path escapes package root: {relative}"
    );
    Ok(absolute)
}
pub(crate) struct LessonData {
    pub(crate) chapters: Vec<crate::Chapter>,
    manifest: Vec<u8>,
    assets: BTreeMap<String, Vec<u8>>,
    ids: BTreeSet<String>,
}
pub(crate) fn read_pin(
    packages: &Path,
    repository: &str,
    manifest: &str,
) -> Result<(wire::SourcePin, Option<LessonData>)> {
    let dir = within(&packages.canonicalize()?, repository)?;
    let body = fs::read(within(&dir, manifest)?)?;
    let mut inputs = vec![digest(&body)];
    let mut overlay = None;
    let commit = match manifest {
        "course.toml" => {
            let decl: crate::Declaration = toml::from_str(std::str::from_utf8(&body)?)?;
            if decl.profile == "mechanics-of-proof" {
                inputs.push(digest(&fs::read(within(&dir, "exercise.toml")?)?));
            }
            decl.revision
        }
        "lessons.toml" => {
            let manifest: LessonsIndex = toml::from_str(std::str::from_utf8(&body)?)?;
            ensure!(
                manifest.format_version == 1,
                "unsupported lesson index version"
            );
            let mut assets = BTreeMap::new();
            let mut ids = BTreeSet::new();
            let mut chapters = Vec::new();
            for lesson in manifest.lessons {
                ensure!(
                    valid_identity(&lesson.id) && ids.insert(lesson.id.clone()),
                    "invalid or duplicate lesson cell"
                );
                for path in [
                    &lesson.rust.starter,
                    &lesson.rust.check,
                    &lesson.lean.starter,
                    &lesson.lean.check,
                ] {
                    if !assets.contains_key(path) {
                        assets.insert(path.clone(), fs::read(within(&dir, path)?)?);
                    }
                }
                let mut blocks = Vec::new();
                if let Some(prompt) = lesson.prompt {
                    blocks.push(crate::Block::Markdown(prompt));
                }
                for (language, path) in [
                    ("rust", &lesson.rust.starter),
                    ("lean", &lesson.lean.starter),
                ] {
                    let source = std::str::from_utf8(&assets[path])
                        .with_context(|| format!("starter is not UTF-8: {path}"))?;
                    blocks.push(crate::Block::Code {
                        language: language.into(),
                        source: source.into(),
                    });
                }
                if let Some(text) = lesson.attribution {
                    blocks.push(crate::Block::Markdown(text));
                }
                if let Some(link) = lesson.further_reading {
                    blocks.push(crate::Block::Markdown(format!("Further reading: <{link}>")));
                }
                chapters.push(crate::Chapter {
                    id: lesson.id.clone(),
                    title: lesson.title.unwrap_or(lesson.id),
                    revision: String::new(),
                    blocks,
                });
            }
            for bytes in assets.values() {
                inputs.push(digest(bytes));
            }
            let commit = digest(&body);
            overlay = Some(LessonData {
                chapters,
                manifest: body,
                assets,
                ids,
            });
            commit
        }
        _ => digest(&body),
    };
    let revision = wire::natural_id(&[
        "quneic.learning.edition",
        repository,
        manifest,
        &inputs.join("/"),
        &commit,
    ]);
    if let Some(data) = &mut overlay {
        let edition = uuid::Uuid::from_slice(&revision.uuid.value)?.to_string();
        for chapter in &mut data.chapters {
            chapter.revision.clone_from(&edition);
        }
    }
    Ok((
        wire::SourcePin {
            repository: Some(repository.into()),
            manifest_path: Some(manifest.into()),
            commit: Some(commit),
            revision: revision.into(),
        },
        overlay,
    ))
}
pub(crate) fn cell_ref(pin: &wire::Ref, name: &str) -> wire::CellRef {
    let cell = wire::natural_id(&[
        "quneic.learning.cell",
        &serde_json::to_string(pin).expect("serialize identity"),
        name,
    ]);
    wire::CellRef {
        tablet_revision: pin.clone().into(),
        cell: cell.into(),
    }
}

#[derive(Deserialize)]
struct LessonsIndex {
    format_version: u32,
    lessons: Vec<LessonId>,
}
#[derive(Deserialize)]
struct LessonPart {
    starter: String,
    check: String,
}
#[derive(Deserialize)]
struct LessonId {
    title: Option<String>,
    prompt: Option<String>,
    attribution: Option<String>,
    further_reading: Option<String>,
    rust: LessonPart,
    lean: LessonPart,
    id: String,
}
impl Catalog {
    pub fn lesson_asset(&self, tablet: &str, path: &str) -> Option<&[u8]> {
        self.lesson_data
            .get(tablet)?
            .assets
            .get(path)
            .map(Vec::as_slice)
    }
    pub fn lesson_manifest(&self, tablet: &str) -> Option<&[u8]> {
        Some(&self.lesson_data.get(tablet)?.manifest)
    }
    pub fn open(root_json: &Path, packages: &Path) -> Result<Self> {
        Self::open_with_git(root_json, packages, crate::source::native_git)
    }
    pub fn open_with_git(
        root_json: &Path,
        packages: &Path,
        git: impl Fn(&Path, &[&str]) -> Result<Vec<u8>>,
    ) -> Result<Self> {
        let root: LearningRoot = serde_json::from_slice(&fs::read(root_json)?)
            .with_context(|| root_json.display().to_string())?;
        Self::from_root(root, packages, git)
    }
    fn from_root(
        root: LearningRoot,
        packages: &Path,
        git: impl Fn(&Path, &[&str]) -> Result<Vec<u8>>,
    ) -> Result<Self> {
        ensure!(root.id.valid(), "invalid LearningRoot id");
        for (alias, r) in &root.aliases {
            ensure!(valid_identity(alias) && r.valid(), "invalid alias {alias}");
        }
        let mut declared = Vec::new();
        for (id, text) in root
            .topics
            .iter()
            .map(|t| (&t.id, &t.label))
            .chain(root.outcomes.iter().map(|o| (&o.id, &o.description)))
        {
            ensure!(
                id.valid()
                    && !text.as_deref().unwrap_or_default().trim().is_empty()
                    && !declared.contains(&id),
                "invalid or duplicate topic/outcome declaration"
            );
            declared.push(id);
        }
        let mut cells = BTreeMap::new();
        let mut lesson_data = BTreeMap::new();
        let mut courses = BTreeMap::new();
        let mut map_paths = BTreeMap::new();
        let packages = packages.canonicalize()?;
        for pin in &root.courses {
            let (actual, overlay) = read_pin(&packages, pin.repo(), pin.manifest())?;
            ensure!(&actual == pin, "stale course pin for {}", pin.repo());
            let aliases: Vec<_> = root
                .aliases
                .iter()
                .filter(|(_, r)| **r == *pin.revision)
                .map(|(k, _)| k.clone())
                .collect();
            ensure!(
                aliases.len() == 1,
                "each course edition needs one unambiguous alias: {}",
                pin.repo()
            );
            let alias = aliases.into_iter().next().unwrap();
            let dir = within(&packages, pin.repo())?;
            let ids = match pin.manifest() {
                "course.toml" => {
                    let course = Course::open_with_git(&dir, &git)?;
                    let ids = course
                        .chapters
                        .iter()
                        .map(|c| c.id.clone())
                        .chain(course.questions.keys().cloned())
                        .collect();
                    courses.insert(alias.clone(), course);
                    ids
                }
                "lessons.toml" => {
                    let data = overlay.context("missing lesson edition")?;
                    let ids = data.ids.clone();
                    lesson_data.insert(alias.clone(), data);
                    ids
                }
                _ => bail!("unsupported course manifest {}", pin.manifest()),
            };
            ensure!(cells.insert(alias, ids).is_none(), "duplicate course pin");
        }
        for pin in &root.maps {
            ensure!(
                pin.revision.valid()
                    && root
                        .aliases
                        .values()
                        .filter(|r| **r == *pin.revision)
                        .count()
                        == 1,
                "map needs one pinned alias"
            );
            ensure!(
                pin.commit_id().len() == 64
                    && pin.commit_id().bytes().all(|b| b.is_ascii_hexdigit()),
                "map requires a SHA-256 pin"
            );
            let dir = within(&packages, pin.repo())?;
            let path = within(&dir, pin.manifest())?;
            ensure!(
                map_paths.insert(path, pin.clone()).is_none(),
                "duplicate map pin"
            );
        }
        let catalog = Self {
            root,
            lesson_data,
            courses,
            cells,
            map_paths,
        };
        let mut exercises = Vec::new();
        for e in &catalog.root.exercises {
            ensure!(
                catalog.cells.iter().any(|(alias, cells)| cells
                    .iter()
                    .any(|name| cell_ref(&catalog.root.aliases[alias], name) == *e.cell)),
                "unresolved root exercise cell"
            );
            ensure!(
                e.checker.valid()
                    && e.completion_rule.valid()
                    && catalog.root.outcomes.iter().any(|o| o.id == e.outcome),
                "invalid exercise checker, completion rule or outcome"
            );
            ensure!(!exercises.contains(&&e.cell), "duplicate root exercise");
            exercises.push(&e.cell);
        }
        Ok(catalog)
    }
    fn resolve(&self, addr: &Address) -> Option<wire::CellRef> {
        self.cells
            .get(&addr.tablet)?
            .contains(&addr.cell)
            .then(|| cell_ref(&self.root.aliases[&addr.tablet], &addr.cell))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedMap {
    pub id: String,
    pub cut: String,
    pub placements: Vec<AuthoredPlacement>,
    pub(crate) wire: LearningMap,
}
impl ResolvedMap {
    pub fn export_toml(&self) -> Result<String> {
        Ok(toml::to_string_pretty(&Export {
            format_version: 1,
            id: &self.id,
            placements: &self.placements,
        })?)
    }
}
/// Read-only adaptation. Successful editions remain accessible at their original cuts.
#[derive(Default)]
pub struct MapStore {
    current: Option<String>,
    editions: BTreeMap<String, ResolvedMap>,
}
impl MapStore {
    pub fn current(&self) -> Option<&ResolvedMap> {
        self.at(self.current.as_deref()?)
    }
    pub fn at(&self, cut: &str) -> Option<&ResolvedMap> {
        self.editions.get(cut)
    }
    pub fn reload(&mut self, catalog: &Catalog, path: &Path) -> Result<&ResolvedMap, Diagnostic> {
        let error = |e| diagnostic(path, "", 0, e);
        let canonical = path.canonicalize().map_err(|e| error(e.to_string()))?;
        let pin = catalog
            .map_paths
            .get(&canonical)
            .ok_or_else(|| error("map path has no pin in this root".into()))?;
        let source = fs::read_to_string(&canonical).map_err(|e| error(e.to_string()))?;
        let map = resolve(catalog, path, &source)?;
        let checksum = digest(source.as_bytes());
        let revision = wire::natural_id(&[
            "quneic.learning.edition",
            pin.repo(),
            pin.manifest(),
            &checksum,
            &checksum,
        ]);
        if pin.commit_id() != checksum || *pin.revision != revision {
            return Err(error(
                "stale map pin; review the edition and update its root pin".into(),
            ));
        }
        let cut = map.cut.clone();
        self.editions.insert(cut.clone(), map);
        self.current = Some(cut.clone());
        Ok(&self.editions[&cut])
    }
}
/// Validate an authoring draft before pinning. This function only returns data.
pub fn resolve(catalog: &Catalog, path: &Path, source: &str) -> Result<ResolvedMap, Diagnostic> {
    let err = |pos, reason: String| diagnostic(path, source, pos, reason);
    let input: Authored = toml::from_str(source)
        .map_err(|e| err(e.span().map_or(0, |s| s.start), e.message().into()))?;
    if *input.format_version.get_ref() != 1 {
        return Err(err(
            input.format_version.span().start,
            "unsupported map version".into(),
        ));
    }
    if !valid_identity(input.id.get_ref()) {
        return Err(err(input.id.span().start, "invalid map id".into()));
    }
    let id = input.id.into_inner();
    let root_id = serde_json::to_string(&catalog.root.id).unwrap();
    let map_id = wire::natural_id(&["quneic.learning.map", &root_id, &id]);
    let mut ids = BTreeSet::new();
    let mut placements = Vec::new();
    let mut resolved = Vec::new();
    for row in input.placements {
        let p = row.into_inner();
        if !valid_identity(p.id.get_ref()) || !ids.insert(p.id.get_ref().clone()) {
            return Err(err(
                p.id.span().start,
                format!("invalid or duplicate placement id {}", p.id.get_ref()),
            ));
        }
        let address = |a: &Spanned<Address>| {
            catalog.resolve(a.get_ref()).ok_or_else(|| {
                err(
                    a.span().start,
                    format!(
                        "unknown pinned cell {}:{}",
                        a.get_ref().tablet,
                        a.get_ref().cell
                    ),
                )
            })
        };
        let at = address(&p.at)?;
        let exercise = address(&p.exercise)?;
        let declaration = catalog
            .root
            .exercises
            .iter()
            .find(|e| *e.cell == exercise)
            .ok_or_else(|| {
                err(
                    p.exercise.span().start,
                    "cell is not a declared exercise".into(),
                )
            })?;
        let topic = catalog
            .root
            .aliases
            .get(p.topic.get_ref())
            .filter(|r| catalog.root.topics.iter().any(|t| *t.id == **r))
            .ok_or_else(|| {
                err(
                    p.topic.span().start,
                    format!("unknown topic {}", p.topic.get_ref()),
                )
            })?;
        let assesses = p
            .assesses
            .as_ref()
            .map(|a| {
                catalog
                    .root
                    .aliases
                    .get(a.get_ref())
                    .filter(|r| **r == *declaration.outcome)
                    .cloned()
                    .ok_or_else(|| {
                        err(
                            a.span().start,
                            format!("outcome {} does not match exercise", a.get_ref()),
                        )
                    })
            })
            .transpose()?;
        resolved.push(wire::Placement {
            id: wire::natural_id(&["quneic.learning.placement", &root_id, &id, p.id.get_ref()])
                .into(),
            at: at.into(),
            exercise: exercise.into(),
            topic: topic.clone().into(),
            assesses: assesses.into(),
        });
        placements.push(AuthoredPlacement {
            id: p.id.into_inner(),
            at: p.at.into_inner(),
            exercise: p.exercise.into_inner(),
            topic: p.topic.into_inner(),
            assesses: p.assesses.map(Spanned::into_inner),
        });
    }
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(&catalog.root).unwrap());
    hasher.update([0]);
    hasher.update(source);
    Ok(ResolvedMap {
        id,
        cut: format!("{:x}", hasher.finalize()),
        placements,
        wire: LearningMap {
            id: map_id.into(),
            placements: resolved,
        },
    })
}
