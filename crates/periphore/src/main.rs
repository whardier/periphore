use clap::Parser;

/// Periphore input sharing CLI
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // Subcommands implemented in Phase 5 via periphore-cli library
}

fn main() -> anyhow::Result<()> {
    let _args = Args::parse();
    // Phase 5: periphore_cli::run(args)
    println!("periphore: CLI not yet implemented. Run `periphored` to start the daemon.");
    Ok(())
}
