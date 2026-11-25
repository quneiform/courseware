use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
pub enum QuestionKind {
    MultipleChoice,
    ShortAnswer,
    Tracing,
}

#[derive(Clone, Debug)]
pub struct Question {
    pub id: String,
    pub kind: QuestionKind,
    pub prompt: String,
    pub starter: Option<String>,
    pub choices: Vec<String>,
    pub multiple: bool,
    pub quiz: String,
}

pub(super) struct Entry {
    pub public: Question,
    key: Key,
    context: String,
}
enum Key {
    Choice(Vec<String>),
    Short {
        answer: String,
        alternatives: Vec<String>,
    },
    Tracing {
        stdout: Option<String>,
    },
}

#[derive(Deserialize)]
struct Document {
    #[serde(default)]
    multipart: BTreeMap<String, String>,
    questions: Vec<RawQuestion>,
}

#[derive(Deserialize)]
struct RawQuestion {
    id: String,
    #[serde(rename = "type")]
    kind: QuestionKind,
    prompt: Prompt,
    answer: Answer,
    #[serde(default)]
    context: String,
    multipart: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Prompt {
    prompt: Option<String>,
    program: Option<String>,
    #[serde(default)]
    distractors: Vec<String>,
    #[serde(default)]
    sort_answers: bool,
    answer_index: Option<usize>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Answer {
    answer: Option<OneOrMany>,
    #[serde(default)]
    alternatives: Vec<String>,
    does_compile: Option<bool>,
    stdout: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

/// CEL uses mdBook's default Rust fences as well as its IDE and Aquascope fences.
fn prompt_program(prompt: &str) -> Option<String> {
    let mut lines = prompt.lines();
    while let Some(line) = lines.next() {
        let Some(info) = line.trim_start().strip_prefix("```") else {
            continue;
        };
        let language = info.split(',').next().unwrap_or("");
        let body = lines.by_ref().take_while(|l| l.trim_end() != "```");
        if !matches!(language, "" | "rust" | "rs" | "ide" | "aquascope") {
            body.for_each(drop);
            continue;
        }
        let mut output = String::new();
        for line in body {
            let line = line.strip_prefix("# ").unwrap_or(line);
            if language == "aquascope" {
                let line = line
                    .strip_prefix(r"\#")
                    .map(|s| format!("#{s}"))
                    .unwrap_or_else(|| line.strip_prefix('#').unwrap_or(line).to_owned());
                output.push_str(&crate::chapter::AQUA_MARK.replace_all(&line, ""));
            } else {
                output.push_str(line);
            }
            output.push('\n');
        }
        if !output.trim().is_empty() {
            return Some(output);
        }
    }
    None
}

pub(super) fn parse(path: &str, toml: &str) -> Result<Vec<Entry>> {
    let doc: Document = toml::from_str(toml).with_context(|| path.to_owned())?;
    let mut entries = Vec::new();
    for raw in doc.questions {
        let p = raw.prompt;
        let a = raw.answer;
        let tracing_program = p.program.clone();
        let mut choices = Vec::new();
        let mut multiple = false;
        let (mut prompt, key) = match raw.kind {
            QuestionKind::ShortAnswer => {
                let Some(OneOrMany::One(answer)) = a.answer else {
                    anyhow::bail!("invalid short answer in {path}")
                };
                (
                    p.prompt.context("missing short answer prompt")?,
                    Key::Short {
                        answer,
                        alternatives: a.alternatives,
                    },
                )
            }
            QuestionKind::MultipleChoice => {
                let correct = match a.answer.context("missing choice key")? {
                    OneOrMany::One(x) => vec![x],
                    OneOrMany::Many(xs) => {
                        multiple = true;
                        xs
                    }
                };
                choices = p.distractors;
                if let Some(index) = p.answer_index {
                    ensure!(index <= choices.len(), "answerIndex out of range");
                    choices.splice(index..index, correct.iter().cloned());
                } else {
                    choices.extend(correct.iter().cloned());
                    if p.sort_answers {
                        choices.sort()
                    } else {
                        // Reproducible permutation across imports, independent of correctness.
                        choices.sort_by_cached_key(|choice| {
                            Sha256::digest(format!("{}\0{choice}", raw.id).as_bytes())
                        });
                    }
                }
                (
                    p.prompt.context("missing choice prompt")?,
                    Key::Choice(correct),
                )
            }
            QuestionKind::Tracing => {
                let compiles = a.does_compile.context("missing doesCompile")?;
                let stdout = if compiles {
                    Some(a.stdout.context("missing tracing output")?)
                } else {
                    None
                };
                let program = p.program.context("missing tracing program")?;
                (
                    format!(
                        "Does this program compile? If it does, what does it print?\n\n```rust\n{}\n```",
                        program.trim_end()
                    ),
                    Key::Tracing { stdout },
                )
            }
        };
        if let Some(id) = raw.multipart {
            let prefix = doc.multipart.get(&id).context("unknown multipart prompt")?;
            prompt = format!("{prefix}\n\n{prompt}");
        }
        let public = Question {
            id: raw.id,
            quiz: path.into(),
            kind: raw.kind,
            starter: tracing_program.or_else(|| prompt_program(&prompt)),
            prompt,
            choices,
            multiple,
        };
        entries.push(Entry {
            public,
            key,
            context: raw.context,
        });
    }
    ensure!(!entries.is_empty(), "empty quiz {path}");
    Ok(entries)
}

impl Entry {
    // Match the grading rules of CEL's mdbook-quiz v0.5.0, pinned by upstream CI.
    pub(super) fn check(&self, response: &[String]) -> bool {
        match &self.key {
            Key::Choice(keys) => {
                let mut actual: Vec<_> = response.iter().collect();
                actual.sort();
                let mut correct: Vec<_> = keys.iter().collect();
                correct.sort();
                actual == correct
            }
            Key::Short {
                answer,
                alternatives,
            } => {
                let [value] = response else { return false };
                let value = value.trim().to_lowercase();
                std::iter::once(answer)
                    .chain(alternatives)
                    .any(|s| s.trim().to_lowercase() == value)
            }
            Key::Tracing {
                stdout: Some(expected),
            } => matches!(response,[mode,out] if mode=="compiles" && out.trim()==expected.trim()),
            Key::Tracing { stdout: None } => matches!(response,[mode] | [mode,_] if mode=="error"),
        }
    }
    pub(super) fn reveal(&self) -> String {
        let answer = match &self.key {
            Key::Choice(xs) => xs.join("\n\n"),
            Key::Short { answer, .. } => answer.clone(),
            Key::Tracing { stdout: Some(out) } => format!(
                "Compiles. Standard output:\n\n```text\n{}\n```",
                out.trim_end()
            ),
            Key::Tracing { stdout: None } => "Does not compile.".to_string(),
        };
        format!("{answer}\n\n{}", self.context)
    }
}
