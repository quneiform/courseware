//! Normalize simple Markdown reference definitions against the edition's pinned mdBook redirects.
use anyhow::{Context, Result, ensure};
use pulldown_cmark::Parser;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
struct Book {
    output: Output,
}
#[derive(Deserialize)]
struct Output {
    html: Html,
}
#[derive(Deserialize)]
struct Html {
    redirect: BTreeMap<String, String>,
}

pub(super) struct Redirects(BTreeMap<String, String>);
impl Redirects {
    pub(super) fn load(config: &[u8], chapters: &BTreeSet<String>) -> Result<Self> {
        let config: Book = toml::from_str(std::str::from_utf8(config)?)?;
        let mut redirects = BTreeMap::new();
        for (old, new) in config.output.html.redirect {
            // Only direct local aliases to enrolled chapters participate. A real
            // published chapter always owns its own name.
            let Some(target) = new.strip_suffix(".html") else {
                continue;
            };
            let Some(alias) = old.strip_suffix(".html") else {
                continue;
            };
            if chapters.contains(target)
                && !chapters.contains(alias)
                && crate::valid_identity(alias)
            {
                redirects.insert(old, new);
            }
        }
        Ok(Self(redirects))
    }

    pub(super) fn apply(&self, markdown: &mut String) -> Result<()> {
        let parser = Parser::new(markdown);
        let mut edits = vec![];
        for (label, def) in parser.reference_definitions().iter() {
            let Some(target) = self.0.get(def.dest.as_ref()) else {
                continue;
            };
            let original = markdown
                .get(def.span.clone())
                .context("invalid reference span")?;
            // Resolve only the destination token. A title or label can contain
            // the same URL; neither changes. Refuse an unfamiliar alias syntax.
            let (written_label, tail) = original
                .split_once("]:")
                .context("unsupported redirect definition")?;
            ensure!(
                written_label
                    .strip_prefix('[')
                    .is_some_and(|name| name.eq_ignore_ascii_case(label)),
                "unsupported redirect label spelling"
            );
            let tail = tail.trim_start();
            let address = tail.strip_prefix('<').unwrap_or(tail);
            ensure!(
                address.starts_with(def.dest.as_ref()),
                "unsupported redirect URL spelling"
            );
            let start = def.span.end - address.len();
            edits.push((start..start + def.dest.len(), target));
        }
        edits.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
        for (span, replacement) in edits {
            markdown.replace_range(span, replacement);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updates_only_enrolled_reference_destinations() {
        let cfg = br#"[output.html.redirect]
"old.html"="new.html"
"stay.html"="new.html"
"web.html"="https://example.com/"
"unknown.html"="missing.html"
"#;
        let links = Redirects::load(cfg, &["new".into(), "stay".into()].into()).unwrap();
        let mut text = "[old.html]: <old.html> \"title]: old.html\"\n[keep]: stay.html\n[web]: web.html\n[unknown]: unknown.html\n\n```\n[example]: old.html\n```\n".to_string();
        let expected = text.replacen("<old.html>", "<new.html>", 1);
        links.apply(&mut text).unwrap();
        assert_eq!(text, expected);
    }
}

#[cfg(test)]
mod pinned_book {
    use super::*;
    use crate::{Block, Course, source};
    use std::path::Path;

    #[test]
    fn only_the_verified_unsafe_rust_reference_changes_across_the_full_edition() -> Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../courses/rust-book");
        let open = |manifest: &str| -> Result<Course> {
            let declaration = std::fs::read_to_string(root.join("editions").join(manifest))?;
            Course::open_cel(&root, toml::from_str(&declaration)?, source::native_git)
        };
        let previous = open("source-anchors.toml")?;
        let successor = open("source-redirects.toml")?;
        assert_eq!(successor.chapters.len(), 120);
        assert_eq!(previous.origins, successor.origins);
        assert!(successor.chapter("ch20-01-unsafe-rust").is_some());
        let mut expected = previous.chapters;
        let mut count = 0;
        for chapter in &mut expected {
            for block in &mut chapter.blocks {
                if let Block::Markdown(md) = block
                    && md.contains("[unsafe]: ch19-01-unsafe-rust.html")
                {
                    assert_eq!(chapter.id, "ch04-03-fixing-ownership-errors");
                    count += md.matches("[unsafe]: ch19-01-unsafe-rust.html").count();
                    *md = md.replace(
                        "[unsafe]: ch19-01-unsafe-rust.html",
                        "[unsafe]: ch20-01-unsafe-rust.html",
                    );
                    assert!(
                        Parser::new(md)
                            .reference_definitions()
                            .get("unsafe")
                            .is_some_and(|d| d.dest.as_ref() == "ch20-01-unsafe-rust.html")
                    );
                }
            }
        }
        assert_eq!(count, 1);
        assert_eq!(expected, successor.chapters);
        Ok(())
    }
}
