use crate::authoring::course_pin;
use crate::mapping::{self, Catalog, MapStore};
use anyhow::{Context, Result, bail, ensure};
use buffa::Message;
use std::{
    env,
    io::{self, Read, Write},
    path::Path,
};
pub fn validate_learning_map() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if let [action, packages, recipe] = args.as_slice()
        && action == "--author-root"
    {
        let source = std::fs::read_to_string(recipe)?;
        let root = crate::authoring::build_root(Path::new(packages), &source)?;
        println!("{}", serde_json::to_string_pretty(&root)?);
        return Ok(());
    }
    if let [action, packages, repository, manifest] = args.as_slice()
        && action == "--pin"
    {
        println!(
            "{}",
            serde_json::to_string_pretty(&course_pin(Path::new(packages), repository, manifest)?)?
        );
        return Ok(());
    }
    let [root, packages, map, rest @ ..] = args.as_slice() else {
        bail!(
            "usage: validate-learning-map ROOT_JSON COURSES_DIR MAP_TOML [--export|--wire|--draft] or --author-root COURSES_DIR ROOT_RECIPE or --pin COURSES_DIR REPOSITORY MANIFEST"
        )
    };
    let catalog = Catalog::open(Path::new(root), Path::new(packages))?;
    if rest == ["--draft"] {
        let source = std::fs::read_to_string(map)?;
        let resolved = mapping::resolve(&catalog, Path::new(map), &source)?;
        println!(
            "{} draft placements · {}",
            resolved.placements.len(),
            resolved.cut
        );
        return Ok(());
    }
    let mut store = MapStore::default();
    let current = store
        .reload(&catalog, Path::new(map))
        .context("validate learning map")?;
    if let [flag, alias, id, public_sha256] = rest
        && flag == "--check-question"
    {
        const LIMIT: u64 = 64 * 1024;
        let mut submission = Vec::new();
        io::stdin()
            .lock()
            .take(LIMIT + 1)
            .read_to_end(&mut submission)?;
        ensure!(
            submission.len() <= LIMIT as usize,
            "oversized question submission"
        );
        let response = buffa_types::google::protobuf::StringValue::decode_from_slice(&submission)?;
        let result = crate::export::material::check_short_answer(
            &catalog,
            alias,
            id,
            public_sha256,
            response.value,
        )?;
        io::stdout().lock().write_all(&result.encode_to_vec())?;
        return Ok(());
    }
    if let [flag, alias, id] = rest {
        let bytes = match flag.as_str() {
            "--chapter-posit" => {
                crate::export::material::chapter(&catalog, alias, id)?.encode_to_vec()
            }
            "--question-cell" => {
                crate::export::material::question_cell(&catalog, alias, id)?.encode_to_vec()
            }
            "--question-wire" => {
                crate::export::material::question(&catalog, alias, id)?.encode_to_vec()
            }
            _ => bail!("unknown course material option"),
        };
        io::stdout().lock().write_all(&bytes)?;
        return Ok(());
    }
    if let [flag, alias] = rest {
        let bytes = match flag.as_str() {
            "--adapted-course" => crate::export::adapted(&catalog, alias)?.encode_to_vec(),
            "--course-posit" => crate::export::material::course(&catalog, alias)?.encode_to_vec(),
            _ => bail!("unknown course option"),
        };
        io::stdout().lock().write_all(&bytes)?;
        return Ok(());
    }
    match rest.first().map(String::as_str) {
        None if rest.is_empty() => {
            println!("{} placements · {}", current.placements.len(), current.cut)
        }
        Some("--root-posit") if rest.len() == 1 => io::stdout()
            .lock()
            .write_all(&crate::export::material::root(&catalog).encode_to_vec())?,
        Some("--root-wire") if rest.len() == 1 => io::stdout()
            .lock()
            .write_all(&catalog.root.encode_to_vec())?,
        Some("--map-wire") if rest.len() == 1 => io::stdout()
            .lock()
            .write_all(&current.wire.encode_to_vec())?,
        Some("--posit-wire") if rest.len() == 1 => io::stdout()
            .lock()
            .write_all(&crate::export::posit(current, &catalog.root.id).encode_to_vec())?,
        Some("--export") if rest.len() == 1 => print!("{}", current.export_toml()?),
        Some("--wire") if rest.len() == 1 => {
            println!("{}", serde_json::to_string_pretty(&current.wire)?)
        }
        _ => bail!("unknown validation option"),
    }
    Ok(())
}
