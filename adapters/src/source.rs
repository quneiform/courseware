use anyhow::{Context, Result, bail, ensure};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Component, Path, PathBuf},
    process::Command,
};

pub(super) type Files = BTreeMap<String, Vec<u8>>;

/// Standalone course roots use Git as an immutable-object reader. An embedding host
/// can supply its own existing effect handler through Course::open_with_git.
#[allow(
    clippy::disallowed_methods,
    reason = "standalone reader's explicit Git effect"
)]
pub(super) fn native_git(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .output()
        .context("open pinned course source with Git")?;
    ensure!(
        output.status.success(),
        "Git source refused: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}

pub(super) fn archive(
    root: &Path,
    revision: &str,
    paths: &[&str],
    run_git: impl FnOnce(&Path, &[&str]) -> Result<Vec<u8>>,
) -> Result<Files> {
    ensure!(
        revision.len() == 40 && revision.bytes().all(|b| b.is_ascii_hexdigit()),
        "expected full Git commit ID"
    );
    ensure!(
        root.join(".git").exists(),
        "course source is unavailable; initialize its upstream submodule"
    );
    // Read immutable objects. A local checkout edit, symlink or untracked quiz key cannot alter the reading.
    let mut args = vec!["archive", "--format=tar", revision, "--"];
    args.extend_from_slice(paths);
    let archive = run_git(root, &args)?;
    let mut files = BTreeMap::new();
    for entry in tar::Archive::new(&archive[..]).entries()? {
        let mut entry = entry?;
        if entry.header().entry_type().is_file() {
            let path = entry
                .path()?
                .to_str()
                .context("non-UTF-8 course path")?
                .to_owned();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            files.insert(path, bytes);
        }
    }
    Ok(files)
}

pub(super) fn relative(base: &str, path: &str) -> Result<String> {
    let mut out = PathBuf::from(base);
    for x in Path::new(path).components() {
        match x {
            Component::Normal(name) => out.push(name),
            Component::CurDir => {}
            Component::ParentDir => ensure!(out.pop(), "course path escapes its source: {path}"),
            _ => bail!("expected relative source path: {path}"),
        }
    }
    Ok(out.to_str().context("non UTF-8 path")?.to_owned())
}

pub(super) fn text<'a>(files: &'a Files, path: &str) -> Result<&'a str> {
    std::str::from_utf8(
        files
            .get(path)
            .with_context(|| format!("missing course file {path}"))?,
    )
    .with_context(|| format!("invalid UTF-8 in {path}"))
}

pub(super) fn include(files: &Files, base: &str, selector: &str) -> Result<String> {
    let (name, span) = selector.split_once(':').unwrap_or((selector, ""));
    let path = relative(base, name)?;
    let lines = text(files, &path)?.lines();
    let region = if span.is_empty() {
        None
    } else if span.contains(':') || span.parse::<usize>().is_ok() {
        let (a, b) = span.split_once(':').unwrap_or((span, span));
        let start = if a.is_empty() {
            1
        } else {
            a.parse().context("invalid include start")?
        };
        let end = if b.is_empty() {
            usize::MAX
        } else {
            b.parse().context("invalid include end")?
        };
        ensure!(
            start > 0 && start <= end,
            "invalid include range {selector}"
        );
        Some((start, end))
    } else {
        None
    };
    let anchored = !span.is_empty() && region.is_none();
    let mut selected = !anchored;
    let mut found = false;
    let mut out = Vec::new();
    for (idx, line) in lines.enumerate() {
        if let Some((_, name)) = line.split_once("ANCHOR_END:") {
            if anchored && name.split_whitespace().next() == Some(span) {
                selected = false;
            }
            continue;
        }
        if let Some((_, name)) = line.split_once("ANCHOR:") {
            if anchored && name.split_whitespace().next() == Some(span) {
                selected = true;
                found = true;
            }
            continue;
        }
        if selected && region.is_none_or(|(a, b)| (a..=b).contains(&(idx + 1))) {
            out.push(line);
        }
    }
    ensure!(!anchored || found, "missing anchor {span} in {path}");
    Ok(out.join("\n"))
}
