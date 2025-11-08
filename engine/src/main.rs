use clap::Parser;
use env_logger::Env;
use log::{error, info};
use solana_client::rpc_client::RpcClient;
use std::collections::HashMap;
use std::sync::Arc;

mod error;
mod liquidation;
mod oracle;
mod position;
mod types;

use crate::{
    error::LiquidationError,
    liquidation::LiquidationEngine,
    oracle::{OracleConfig, PythOracle},
    types::LiquidationConfig,
};

// Re-export error type for use in main
pub use error::LiquidationError as Error;

/// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Solana RPC URL
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    /// Path to payer keypair file (default: ./local_keypair.json)
    #[arg(long, default_value = "./local_keypair.json")]
    keypair: String,

    /// Run in dry-run mode (no actual transactions)
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Check interval in milliseconds
    #[arg(long, default_value_t = 1000)]
    check_interval_ms: u64,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or(&args.log_level)).init();

    info!("Starting liquidation engine with config: {:?}", args);

    // Initialize RPC client
    let rpc_client = Arc::new(RpcClient::new(&args.rpc_url));

    // Initialize oracle with default config
    let oracle = Arc::new(PythOracle::new(
        &args.rpc_url,
        HashMap::new(), // You might want to load price accounts from config
        Some(OracleConfig {
            max_price_age_secs: 60, // 1 minute
            min_confidence_interval: 0.05, // 5%
            max_confidence_interval: 0.1, // 10% (as a decimal, not seconds)
            use_mainnet: false,
        }),
    ));

    // Create liquidation engine with default config and override specific fields
    let mut config = LiquidationConfig::default();
    config.check_interval_ms = args.check_interval_ms;
    
    let engine = LiquidationEngine::new(
        rpc_client,
        oracle,
        config,
    );
    
    info!("Liquidation engine started with config: {:?}", engine.config());

    // Start the engine
    engine.start().await.map_err(|e| {
        error!("Engine error: {}", e);
        e
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::Keypair;

    #[test]
    fn test_config_default() {
        let config = LiquidationConfig::default();
        assert_eq!(config.check_interval_ms, 1000);
        assert_eq!(config.maintenance_margin, 0.05);
    }

    #[test]
    fn test_engine_initialization() {
        let rpc_client = Arc::new(RpcClient::new("https://api.devnet.solana.com"));
        let oracle = Arc::new(PythOracle::new(
            "https://api.devnet.solana.com",
            HashMap::new(),
            None,
        ));
        let config = LiquidationConfig::default();
        let _engine = LiquidationEngine::new(rpc_client, oracle, config);
    }
}
