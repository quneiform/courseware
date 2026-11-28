//! Import the published Sphinx contents and reading excerpts directly from the pinned Git tree.
use super::{Block, Chapter, Course, Declaration, NavigationNode, source};
use anyhow::{Context, Result, ensure};
use regex::Regex;
use serde::Deserialize;
use std::{collections::BTreeSet, path::Path, sync::LazyLock};

const SITE: &str = "https://hrmacbeth.github.io/math2001";
const REPO: &str = "https://github.com/hrmacbeth/math2001";

static TOC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<li class="toctree-l([12])"><a class="reference internal" href="([a-zA-Z0-9_]+\.html(?:#[a-zA-Z0-9-]+)?)">([^<]+)</a>"#).unwrap()
});
static PARAGRAPH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<p>(.*?)</p>").unwrap());
static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());

static SECTION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"</?section(?: [^>]*)?>").unwrap());
static ENTITY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"&#(\d+);").unwrap());

fn excerpt(html: &str, anchor: &str) -> Result<Vec<Block>> {
    let marker = if anchor.is_empty() {
        "<section id=\"".to_owned()
    } else {
        format!("<section id=\"{anchor}\">")
    };
    let (_, body) = html.split_once(&marker).context("Missing Sphinx section")?;
    let mut depth = 1usize;
    let mut end = None;
    for tag in SECTION.find_iter(body) {
        if tag.as_str().starts_with("</") {
            depth -= 1;
        } else {
            depth += 1;
        }
        if depth == 0 {
            end = Some(tag.start());
            break;
        }
    }
    let section = &body[..end.context("Unclosed Sphinx section")?];
    Ok(PARAGRAPH
        .captures_iter(section)
        .take(2)
        .map(|p| Block::Markdown(text(&p[1])))
        .collect())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Exercise {
    id: String,
    chapter: String,
    path: String,
    first_line: usize,
    last_line: usize,
    toolchain: String,
    author_starter: Option<AuthorStarter>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthorStarter {
    id: String,
    chapter: String,
    title: String,
    author: String,
    prompt: String,
    source: String,
}

fn text(html: &str) -> String {
    let decoded = ENTITY.replace_all(html, |c: &regex::Captures| {
        c[1].parse()
            .ok()
            .and_then(char::from_u32)
            .unwrap_or('�')
            .to_string()
    });
    TAG.replace_all(&decoded, "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
        .replace(r"\(", "`")
        .replace(r"\)", "`")
}

pub(super) fn open(
    root: &Path,
    spec: Declaration,
    git: impl FnOnce(&Path, &[&str]) -> Result<Vec<u8>>,
) -> Result<Course> {
    let ex: Exercise = toml::from_str(&std::fs::read_to_string(root.join("exercise.toml"))?)?;
    let files = source::archive(
        &root.join("upstream"),
        &spec.revision,
        &["html", "Math2001", "lean-toolchain"],
        git,
    )?;
    ensure!(
        source::text(&files, "lean-toolchain")?.trim() == ex.toolchain,
        "Exercise toolchain differs from the pinned edition"
    );
    let lean = source::text(&files, &ex.path)?;
    let lines: Vec<_> = lean.lines().collect();
    let starter = ex
        .first_line
        .checked_sub(1)
        .and_then(|first| lines.get(first..ex.last_line))
        .context("Missing pinned exercise")?;
    ensure!(
        starter.len() == 2
            && starter[0].starts_with("example ")
            && starter[0].ends_with(":= by")
            && starter[1].trim() == "sorry",
        "Pinned starter statement changed"
    );
    let mut course = Course::empty(spec);
    let index = source::text(&files, &course.navigation.source)?
        .split_once("<div role=\"main\"")
        .context("Missing Sphinx index")?
        .1;
    let mut ids = BTreeSet::new();
    for entry in TOC.captures_iter(index) {
        let target = &entry[2];
        let (filename, anchor) = target.split_once('#').unwrap_or((target, ""));
        let parent = filename
            .trim_end_matches(".html")
            .replace('_', "-")
            .to_ascii_lowercase();
        let id = if anchor.is_empty() {
            parent.clone()
        } else {
            format!("{parent}--{anchor}")
        };
        ensure!(ids.insert(id.clone()), "Duplicate contents entry {target}");
        let title = text(&entry[3]);
        let node = NavigationNode {
            chapter_id: id.clone(),
            title: title.clone(),
            children: Vec::new(),
        };
        if &entry[1] == "1" {
            ensure!(
                anchor.is_empty(),
                "Top-level contents entry must name a page"
            );
            course.navigation.roots.push(node);
        } else {
            let root = course
                .navigation
                .roots
                .last_mut()
                .context("Section without a chapter")?;
            ensure!(
                root.chapter_id == parent && !anchor.is_empty(),
                "Section does not belong to its chapter"
            );
            root.children.push(node);
        }
        let html = source::text(&files, &format!("html/{filename}"))?;
        let mut blocks = excerpt(html, anchor).with_context(|| format!("Read {target}"))?;
        blocks.push(Block::Markdown(format!(
            "[Read the full section in Heather Macbeth’s book]({SITE}/{target})."
        )));
        if id == ex.chapter {
            blocks.push(Block::Markdown(format!("**Proof exercise · {}**\n\nReplace the unfinished proof in this [upstream exercise]({REPO}/blob/{}/{}#L{}-L{}).",ex.id,course.revision,ex.path,ex.first_line,ex.last_line)));
            blocks.push(Block::Code {
                language: "lean4".into(),
                source: format!("{}\n", starter.join("\n")),
            });
        }
        if let Some(extra) = &ex.author_starter
            && extra.chapter == id
        {
            ensure!(
                crate::valid_identity(&extra.id)
                    && [&extra.title, &extra.author, &extra.prompt, &extra.source]
                        .iter()
                        .all(|s| !s.trim().is_empty()),
                "Incomplete authored starter"
            );
            blocks.push(Block::Markdown(format!(
                "**{}**\n\n{}\n\nEditable practice · {}.",
                extra.title, extra.prompt, extra.author
            )));
            blocks.push(Block::Code {
                language: "lean4".into(),
                source: extra.source.clone(),
            });
        }
        course.chapters.push(Chapter {
            id,
            title,
            revision: course.revision.clone(),
            blocks,
        });
    }
    ensure!(
        ids.contains(&ex.chapter)
            && ids.contains(&course.workspace.default_chapter)
            && ex
                .author_starter
                .as_ref()
                .is_none_or(|x| ids.contains(&x.chapter)),
        "Missing exercise section or opening chapter"
    );
    Ok(course)
}
