use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to config file
    #[arg(short, long)]
    config: Option<PathBuf>,
    
    /// Run in dry-run mode (no actual transactions)
    #[arg(long)]
    dry_run: bool,
    
    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    // Initialize logger
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(&args.log_level)
    ).init();
    
    log::info!("Starting liquidation CLI...");
    log::info!("Dry run: {}", args.dry_run);
    
    // TODO: Implement CLI functionality
    
    Ok(())
}
