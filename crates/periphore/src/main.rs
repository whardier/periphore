use clap::Parser;

/// Periphore input sharing CLI.
///
/// Interact with a running `periphored` daemon. If the daemon is not running,
/// most commands will fail with a clear error message.
///
/// Start the daemon first: `periphored [--config FILE]`
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // Subcommands are implemented in Phase 5 via the periphore-cli library.
    // See ROADMAP.md Phase 5 for the planned command surface.
}

fn main() -> anyhow::Result<()> {
    let _args = Args::parse();
    // Phase 5: periphore_cli::run(args)
    // For now: inform the user about the current state.
    eprintln!("periphore: CLI subcommands not yet implemented.");
    eprintln!("Run `periphored` to start the daemon.");
    eprintln!("Use `periphored --help` for daemon options.");
    Ok(())
}
