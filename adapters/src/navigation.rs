//! mdBook's ordered summary supplies chapter identities and parent-child links.
use super::source;
use anyhow::{Context, Result, ensure};
use regex::Regex;
use serde::Deserialize;
use std::{collections::BTreeSet, path::Path};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Navigation {
    pub id: String,
    pub title: String,
    pub kind: String,
    /// Relative to the pinned upstream Git tree.
    pub source: String,
    #[serde(skip)]
    pub roots: Vec<NavigationNode>,
}
#[derive(Debug)]
pub struct NavigationNode {
    pub chapter_id: String,
    pub title: String,
    pub children: Vec<Self>,
}
impl Default for Navigation {
    /// CEL's original format-1 declaration selected the standard summary implicitly.
    fn default() -> Self {
        Self {
            id: "contents".into(),
            title: "Contents".into(),
            kind: "qn.ux.v05.TreeNavigation".into(),
            source: "src/SUMMARY.md".into(),
            roots: Vec::new(),
        }
    }
}

impl Navigation {
    pub(super) fn load(&mut self, files: &source::Files) -> Result<()> {
        ensure!(
            super::valid_identity(&self.id),
            "invalid course navigation identity"
        );
        ensure!(
            !self.title.trim().is_empty(),
            "course navigation requires a title"
        );
        ensure!(
            self.kind == "qn.ux.v05.TreeNavigation",
            "unsupported course navigation kind: {}",
            self.kind
        );
        let source = source::relative("", &self.source)?;
        ensure!(
            source.starts_with("src/") && source.ends_with(".md"),
            "expected an upstream navigation source under src/"
        );
        let base = Path::new(&source)
            .parent()
            .and_then(Path::to_str)
            .context("invalid navigation source")?;
        let summary = source::text(files, &source)?;
        let link = Regex::new(r"^( *)(?:- +)?\[(.+)\]\(([^)]+\.md)\)\s*$")?;
        let mut ids = BTreeSet::new();
        let mut stack: Vec<(usize, NavigationNode)> = Vec::new();
        for (index, line) in summary.lines().enumerate() {
            if line.trim().is_empty() || line.starts_with('#') || line.trim() == "---" {
                continue;
            }
            let loc = || format!("{source}:{}: unsupported entry", index + 1);
            let entry = link.captures(line).with_context(loc)?;
            let indent = entry[1].len();
            let path = source::relative(base, &entry[3])?;
            let id = path
                .strip_prefix("src/")
                .and_then(|s| s.strip_suffix(".md"))
                .filter(|s| !s.is_empty())
                .with_context(loc)?;
            ensure!(ids.insert(id.to_owned()), "duplicate chapter {id}");
            while stack.last().is_some_and(|(parent, _)| *parent >= indent) {
                finish_node(&mut stack, &mut self.roots);
            }
            ensure!(
                indent == 0 || !stack.is_empty(),
                "orphaned child at {source}:{}",
                index + 1
            );
            ensure!(stack.len() < 64, "navigation nesting exceeds 64 levels");
            stack.push((
                indent,
                NavigationNode {
                    chapter_id: id.into(),
                    title: entry[2].into(),
                    children: vec![],
                },
            ));
        }
        while !stack.is_empty() {
            finish_node(&mut stack, &mut self.roots);
        }
        ensure!(!self.roots.is_empty(), "empty course");
        Ok(())
    }

    /// Chapters in the upstream reading order, including linked parent pages.
    pub fn chapters(&self) -> impl Iterator<Item = &NavigationNode> {
        let mut stack: Vec<_> = self.roots.iter().rev().collect();
        std::iter::from_fn(move || {
            let next = stack.pop()?;
            stack.extend(next.children.iter().rev());
            Some(next)
        })
    }
}

fn finish_node(stack: &mut Vec<(usize, NavigationNode)>, roots: &mut Vec<NavigationNode>) {
    let (_, node) = stack.pop().expect("pending summary node");
    match stack.last_mut() {
        Some((_, parent)) => parent.children.push(node),
        None => roots.push(node),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn format_one_declarations_without_navigation_keep_the_original_contents() -> Result<()> {
        let spec: crate::Declaration =
            toml::from_str("format_version=1\nprofile='cel-rust-book'\nrevision='pin'")?;
        assert_eq!(spec.tablet.id, "rust-book");
        assert_eq!(spec.tablet.title, "Rust Book");
        let mut nav = spec.navigation;
        assert_eq!(nav.id, "contents");
        assert_eq!(nav.source, "src/SUMMARY.md");
        let mut files = source::Files::new();
        files.insert("src/SUMMARY.md".into(), b"[First](first.md)".to_vec());
        nav.load(&files)?;
        assert_eq!(nav.roots[0].chapter_id, "first");
        Ok(())
    }
    #[test]
    fn declared_summary_resolves_nested_links_against_its_own_directory() -> Result<()> {
        let mut nav: Navigation = toml::from_str(
            r#"id="outline"
title="Course map"
kind="qn.ux.v05.TreeNavigation"
source="src/course/outline.md""#,
        )?;
        let mut files = source::Files::new();
        files.insert(nav.source.clone(),b"[Introduction](intro.md)\n- [Part](part.md)\n  - [Chapter](chap.md)\n    - [Exercise](../ex.md)\n- [End](end.md)".to_vec());
        nav.load(&files)?;
        assert_eq!(nav.title, "Course map");
        assert_eq!(nav.roots.len(), 3);
        assert_eq!(nav.roots[1].children[0].children[0].chapter_id, "ex");
        assert_eq!(
            nav.chapters()
                .map(|n| n.chapter_id.as_str())
                .collect::<Vec<_>>(),
            [
                "course/intro",
                "course/part",
                "course/chap",
                "ex",
                "course/end"
            ]
        );
        Ok(())
    }
}
