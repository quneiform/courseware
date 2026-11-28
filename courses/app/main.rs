//! This target selects the connected Learning root and the versioned Tablet app template.
#[path = "../../../std/tablet/app/templates/tablet_as_app.rs"]
mod tablet_as_app;
fn main() -> anyhow::Result<()> {
    let declaration = include_bytes!("../connected-lessons/root.json");
    let root: serde_json::Value = serde_json::from_slice(declaration)?;
    let roster = root["courses"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing course roster"))?
        .iter()
        .map(|item| {
            item["repository"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing course repository"))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    tablet_as_app::run(declaration, &roster)
}
