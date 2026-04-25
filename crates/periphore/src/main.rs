use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    periphore_cli::run(periphore_cli::Cli::parse()).await
}
