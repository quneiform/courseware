use crate::{
    Block, quiz,
    source::{self, Files},
};
use anyhow::{Context, Result, ensure};
use regex::{Captures, Regex};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path, sync::LazyLock};

static COMMENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<!--.*?-->").unwrap());
static IMG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?s)<img\b[^>]*?/?>"#).unwrap());
static ATTR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"([\w-]+)\s*=\s*"([^"]*)""#).unwrap());
static PERM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@Perm(?:\[([^\]]+)\])?\{([^}]*)\}").unwrap());
static LISTING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<Listing\b[^>]*>").unwrap());
pub(super) static AQUA_MARK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`(?:\[\]|\{\}|\(focus[^)`]*\))`").unwrap());

static LABELED_SPAN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<span class="(?:filename|caption)">(.*?)</span>"#).unwrap());
static HTML_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<a href="([^"]+)">([^<>]*)</a>"#).unwrap());
static LAYOUT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^\s*(?:<div style="display: flex;[^"]*">|</div>)\s*$"#).unwrap()
});
static INLINE_PERMISSION_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<blockquote><div style="[^"]*"><i>Note:</i> when the expected permissions.*?</div></blockquote>"#).unwrap()
});

fn prose(raw: &str) -> String {
    let raw = COMMENT.replace_all(raw, "");
    let raw = LABELED_SPAN.replace_all(&raw, "$1");
    let raw = HTML_LINK.replace_all(&raw, "[$2]($1)");
    let raw = LAYOUT.replace_all(&raw, "\n");
    let raw = INLINE_PERMISSION_KEY.replace_all(&raw, "> **Note:** CEL abbreviates expected read and write permissions as dots in its graphical diagrams. The permission steps here spell out each change.");
    let raw = raw
        .replace(r#"<i class="fa fa-arrow-turn-up"></i>"#, "↱")
        .replace(r#"<i class="fa fa-arrow-right"></i>"#, "→")
        .replace(r#"<i class="fa fa-rotate-left"></i>"#, "↶")
        .replace(r#"<i class="fa fa-arrow-turn-down"></i>"#, "↳")
        .replace("}@Perm", "}, @Perm");
    let raw = IMG.replace_all(&raw, |m: &Captures<'_>| {
        let attrs: BTreeMap<_, _> = ATTR
            .captures_iter(&m[0])
            .map(|c| (c[1].to_string(), c[2].to_string()))
            .collect();
        let alt = attrs
            .get("alt")
            .map(String::as_str)
            .unwrap_or("Illustration")
            .replace(']', r"\]");
        format!(
            "![{alt}]({})",
            attrs.get("src").map(String::as_str).unwrap_or("")
        )
    });
    let raw = LISTING.replace_all(&raw, |m: &Captures<'_>| {
        let attrs: BTreeMap<_, _> = ATTR
            .captures_iter(&m[0])
            .map(|c| (c[1].to_string(), c[2].to_string()))
            .collect();
        let title = attrs
            .get("number")
            .map(|n| format!("Listing {n}"))
            .unwrap_or_else(|| "Listing".into());
        let file = attrs
            .get("file-name")
            .map(|s| format!(" · `{s}`"))
            .unwrap_or_default();
        let caption = attrs
            .get("caption")
            .map(|s| format!(" — {s}"))
            .unwrap_or_default();
        format!("\n\n**{title}**{file}{caption}\n\n")
    });
    let raw = raw
        .replace("</Listing>", "")
        .replace("<figure>", "")
        .replace("</figure>", "")
        .replace("<figcaption>", "")
        .replace("</figcaption>", "");
    PERM.replace_all(&raw, |m: &Captures<'_>| {
        m.get(1)
            .map(|status| format!("**{} {}**", status.as_str(), &m[2]))
            .unwrap_or_else(|| format!("**{}**", &m[2]))
    })
    .into_owned()
}

fn flush(text: &mut String, blocks: &mut Vec<Block>) {
    if !text.trim().is_empty() {
        let text = prose(text);
        if !text.trim().is_empty() {
            blocks.push(Block::Markdown(text))
        }
    }
    text.clear();
}

fn directive<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let body = line.trim().strip_prefix("{{#")?.strip_suffix("}}")?;
    let (kind, path) = body.split_once(char::is_whitespace)?;
    (kind == name).then(|| path.trim())
}

fn expand(files: &Files, base: &str, line: &str) -> Result<String> {
    let listing = directive(line, "include").or_else(|| directive(line, "rustdoc_include"));
    if let Some(reference) = listing {
        source::include(files, base, reference)
    } else {
        Ok(line.to_string())
    }
}

#[derive(Deserialize)]
struct PermissionTrace {
    steps: Vec<(String, String)>,
}

type PermissionTraces = BTreeMap<String, BTreeMap<String, PermissionTrace>>;

static TRACES: LazyLock<PermissionTraces> = LazyLock::new(|| {
    toml::from_str(include_str!("../data/permissions.toml"))
        .expect("checked-in CEL permission traces")
});

fn permission_steps(path: &str, source: &str) -> Option<Block> {
    let digest = format!("{:x}", Sha256::digest(source.as_bytes()));
    let trace = TRACES.get(path)?.get(&digest)?;
    Some(Block::Steps {
        title: "Permissions by line".into(),
        steps: trace.steps.clone(),
    })
}

pub(super) fn parse(
    path: &str,
    files: &Files,
    questions: &mut BTreeMap<String, quiz::Entry>,
) -> Result<Vec<Block>> {
    let chapter = source::text(files, path)?;
    let base = Path::new(path)
        .parent()
        .and_then(Path::to_str)
        .context("invalid chapter path")?;
    let mut blocks = Vec::new();
    let mut text = String::new();
    let mut lines = chapter.lines().enumerate().peekable();
    let mut in_comment = false;
    while let Some((idx, line)) = lines.next() {
        let trimmed = line.trim_start();
        if in_comment || trimmed.starts_with("<!--") {
            in_comment = !line.contains("-->");
            continue;
        }
        if let Some(reference) = directive(line, "quiz") {
            flush(&mut text, &mut blocks);
            let file = source::relative(base, reference)?;
            for entry in quiz::parse(&file, source::text(files, &file)?)
                .with_context(|| format!("{path}:{}: {file}", idx + 1))?
            {
                let id = entry.public.id.clone();
                ensure!(
                    questions.insert(id.clone(), entry).is_none(),
                    "duplicate question {id}"
                );
                blocks.push(Block::Interaction(id));
            }
            continue;
        }
        if let Some(info) = line.strip_prefix("```") {
            // Course sources use triple-backtick fences at column zero.
            flush(&mut text, &mut blocks);
            let mut code = Vec::new();
            let mut closed = false;
            let lang = info.split(',').next().unwrap_or("");
            for (line_index, line) in lines.by_ref() {
                if line.trim_end() == "```" {
                    closed = true;
                    break;
                }
                let expansion = expand(files, base, line)
                    .with_context(|| format!("{path}:{}", line_index + 1))?;
                for l in expansion.lines() {
                    let l = if lang == "rust" || lang == "aquascope" {
                        if l.trim_start().starts_with("# ")
                            || l.trim() == "#"
                            || (lang == "aquascope" && l.starts_with('#'))
                        {
                            continue;
                        }
                        if lang == "aquascope" {
                            let l = l
                                .strip_prefix(r"\#")
                                .map(|rest| format!("#{rest}"))
                                .unwrap_or_else(|| l.to_string());
                            AQUA_MARK.replace_all(&l, "").into_owned()
                        } else {
                            l.to_string()
                        }
                    } else {
                        l.to_owned()
                    };
                    code.push(l)
                }
                if expansion.is_empty() {
                    code.push(String::new())
                }
            }
            ensure!(closed, "{path}:{}: unclosed code fence", idx + 1);
            let source = code.join("\n");
            let aquascope = lang == "aquascope";
            let steps = if aquascope && info.split(',').any(|opt| opt == "stepper") {
                permission_steps(path, &source)
            } else {
                None
            };
            blocks.push(Block::Code {
                language: if aquascope { "rust" } else { lang }.into(),
                source,
            });
            if let Some(steps) = steps {
                blocks.push(steps)
            } else if aquascope {
                blocks.push(Block::Notice(
                    "Aquascope visualization unavailable for this example.".into(),
                ));
            }
            continue;
        }
        text.push_str(&expand(files, base, line).with_context(|| format!("{path}:{}", idx + 1))?);
        text.push('\n');
    }
    flush(&mut text, &mut blocks);
    Ok(blocks)
}
