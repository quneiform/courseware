//! Load declared course packages into portable reading data for a host application.
mod chapter;
mod cs2023;
mod mechanics;
mod navigation;
mod quiz;
mod source;

use anyhow::{Context, Result, bail, ensure};
pub use navigation::{Navigation, NavigationNode};
mod reading;
pub use quiz::{Question, QuestionKind};
pub use reading::{Block, Chapter};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};

/// The selectable Tablet described by this package.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tablet {
    pub id: String,
    pub title: String,
}

impl Default for Tablet {
    fn default() -> Self {
        Self {
            id: "rust-book".into(),
            title: "Rust Book".into(),
        }
    }
}

fn valid_identity(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
}

/// Which workspace a host can present when this package is selected.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub language: String,
    pub editor: String,
    pub default_chapter: String,
}
impl Default for Workspace {
    fn default() -> Self {
        Self {
            language: "rust".into(),
            editor: "exercise".into(),
            default_chapter: "ch01-02-hello-world".into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Declaration {
    format_version: u32,
    profile: String,
    revision: String,
    #[serde(default)]
    tablet: Tablet,
    #[serde(default)]
    navigation: Navigation,
    #[serde(default)]
    workspace: Workspace,
}

pub struct Course {
    pub revision: String,
    pub tablet: Tablet,
    pub navigation: Navigation,
    pub chapters: Vec<Chapter>,
    pub workspace: Workspace,
    questions: BTreeMap<String, quiz::Entry>,
    assets: BTreeMap<String, Vec<u8>>,
}

impl Course {
    pub fn open(root: &Path) -> Result<Self> {
        Self::open_with_git(root, source::native_git)
    }

    pub fn open_with_git(
        root: &Path,
        run_git: impl FnOnce(&Path, &[&str]) -> Result<Vec<u8>>,
    ) -> Result<Self> {
        let config = fs::read_to_string(root.join("course.toml"))
            .with_context(|| format!("read {}/course.toml", root.display()))?;
        let spec: Declaration = toml::from_str(&config).context("course.toml")?;
        ensure!(spec.format_version == 1, "unsupported course format");
        ensure!(
            valid_identity(&spec.tablet.id) && !spec.tablet.title.trim().is_empty(),
            "invalid course Tablet identity or title"
        );
        ensure!(
            matches!(spec.workspace.editor.as_str(), "exercise" | "reading")
                && !spec.workspace.default_chapter.is_empty(),
            "invalid course workspace"
        );
        let course = match spec.profile.as_str() {
            "cel-rust-book" => Self::open_cel(root, spec, run_git),
            "mechanics-of-proof" => mechanics::open(root, spec, run_git),
            "cs2023" => cs2023::open(root, spec, run_git),
            profile => bail!("unsupported course profile: {profile}"),
        }?;
        ensure!(
            course.chapter(&course.workspace.default_chapter).is_some(),
            "unknown default chapter: {}",
            course.workspace.default_chapter
        );
        Ok(course)
    }

    fn empty(spec: Declaration) -> Self {
        Self {
            tablet: spec.tablet,
            navigation: spec.navigation,
            revision: spec.revision,
            workspace: spec.workspace,
            chapters: Vec::new(),
            questions: BTreeMap::new(),
            assets: BTreeMap::new(),
        }
    }

    fn open_cel(
        root: &Path,
        spec: Declaration,
        run_git: impl FnOnce(&Path, &[&str]) -> Result<Vec<u8>>,
    ) -> Result<Self> {
        let files = source::archive(
            &root.join("upstream"),
            &spec.revision,
            &["src", "quizzes", "listings"],
            run_git,
        )?;
        let mut course = Self::empty(spec);
        course.navigation.load(&files)?;
        for node in course.navigation.chapters() {
            let path = format!("src/{}.md", node.chapter_id);
            let blocks = chapter::parse(&path, &files, &mut course.questions)
                .with_context(|| path.clone())?;
            course.chapters.push(Chapter {
                id: node.chapter_id.clone(),
                title: node.title.clone(),
                revision: course.revision.clone(),
                blocks,
            });
        }
        for (path, bytes) in files {
            if let Some(asset) = path.strip_prefix("src/")
                && asset.starts_with("img/")
            {
                course.assets.insert(asset.to_owned(), bytes);
            }
        }
        Ok(course)
    }

    pub fn chapter(&self, id: &str) -> Option<&Chapter> {
        self.chapters.iter().find(|x| x.id == id)
    }
    pub fn question(&self, id: &str) -> Option<&Question> {
        self.questions.get(id).map(|q| &q.public)
    }
    pub fn check(&self, id: &str, answers: &[String]) -> Result<bool> {
        Ok(self
            .questions
            .get(id)
            .context("unknown question")?
            .check(answers))
    }
    pub fn reveal(&self, id: &str) -> Result<String> {
        Ok(self.questions.get(id).context("unknown question")?.reveal())
    }

    /// Only image bytes already read at the pinned revision can be served.
    pub fn asset(&self, path: &str) -> Result<(String, Vec<u8>)> {
        let bytes = self.assets.get(path).context("unknown course asset")?;
        let mime = match path.rsplit('.').next().unwrap_or("") {
            "svg" => "image/svg+xml",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            _ => bail!("unsupported asset format"),
        };
        Ok((mime.to_owned(), bytes.clone()))
    }
}
