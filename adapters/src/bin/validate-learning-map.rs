#[path = "../mapping/authoring.rs"]
mod authoring;
#[path = "../mapping/export.rs"]
mod export;
include!("../adapter.rs");
#[path = "../archive_session.rs"]
mod archive_session;
#[path = "../mapping/command.rs"]
mod command;
fn main() -> anyhow::Result<()> {
    // qm launches from the Qrate directory and preserves the caller's working directory.
    if let Some(cwd) = std::env::var_os("QM_INVOCATION_CWD") {
        std::env::set_current_dir(cwd)?;
    }
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|v| v == "--archive-session-v1") {
        if args.len() == 2 && args[1] == "--location" {
            println!("{}", std::env::current_exe()?.display());
            return Ok(());
        }
        anyhow::ensure!(args.len() == 1, "unexpected archive session arguments");
        return archive_session::serve();
    }
    command::validate_learning_map()
}
