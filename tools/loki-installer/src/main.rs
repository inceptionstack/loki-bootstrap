mod adapters;
mod cli;
mod core;
mod tui;

use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    cli::run().await
}
