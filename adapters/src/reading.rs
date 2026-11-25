//! Host-independent data for one reading. Identifiers are scoped to a course package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub revision: String,
    pub blocks: Vec<Block>,
}

/// Ordered reading elements. Interaction IDs resolve against the course's own answer checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Markdown(String),
    Code {
        language: String,
        source: String,
    },
    Interaction(String),
    Notice(String),
    Steps {
        title: String,
        steps: Vec<(String, String)>,
    },
}
