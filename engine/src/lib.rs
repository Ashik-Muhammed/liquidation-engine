//! Liquidation Engine for High-Leverage Perpetual Futures Exchange
//! 
//! This module provides real-time monitoring and liquidation of undercollateralized positions
//! in a high-leverage perpetual futures trading environment.

mod error;
mod oracle;
mod position;
mod types;

pub use error::LiquidationError;
pub use types::*;
pub use position::Position;
pub use oracle::OracleProvider;

use log::{info, error};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

/// Main LiquidationEngine that monitors and liquidates undercollateralized positions
pub struct LiquidationEngine {
    /// Solana RPC client for interacting with the blockchain
    rpc_client: Arc<RpcClient>,
    /// Cache of positions being monitored
    positions: RwLock<HashMap<Pubkey, Position>>,
    /// Oracle service for fetching price feeds
    oracle: Arc<dyn OracleProvider + Send + Sync>,
    /// Configuration parameters
    config: LiquidationConfig,
}

/// Configuration for the LiquidationEngine
#[derive(Clone, Debug)]
pub struct LiquidationConfig {
    /// How often to check positions (in seconds)
    pub check_interval_secs: u64,
    /// Minimum time between liquidations for the same position (in seconds)
    pub liquidation_cooldown_secs: i64,
    /// Maximum number of positions to process in one batch
    pub max_batch_size: usize,
    /// Maximum number of concurrent liquidations
    pub max_concurrent_liquidations: usize,
}

impl Default for LiquidationConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 1,
            liquidation_cooldown_secs: 60, // 1 minute cooldown
            max_batch_size: 100,
            max_concurrent_liquidations: 10,
        }
    }
}

impl LiquidationEngine {
    /// Create a new LiquidationEngine instance
    pub fn new(
        rpc_client: Arc<RpcClient>,
        oracle: Arc<dyn OracleProvider + Send + Sync>,
        config: Option<LiquidationConfig>,
    ) -> Self {
        Self {
            rpc_client,
            positions: RwLock::new(HashMap::new()),
            oracle,
            config: config.unwrap_or_default(),
        }
    }

    /// Start the liquidation monitoring service
    pub async fn start(&self) -> Result<(), LiquidationError> {
        info!("Starting liquidation engine...");
        
        let mut interval = interval(Duration::from_secs(self.config.check_interval_secs));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.check_positions().await {
                error!("Error checking positions: {}", e);
                continue;
            }
        }
    }
    
    /// Check all monitored positions for liquidation
    pub async fn check_positions(&self) -> Result<(), LiquidationError> {
        info!("Checking all positions for liquidation");
        
        // Get a snapshot of all positions
        let positions = self.positions.read().await;
        let positions_snapshot: Vec<Position> = positions.values().cloned().collect();
        drop(positions); // Release the read lock
        
        // Process positions sequentially to avoid borrow checker issues
        for position in positions_snapshot {
            if let Err(e) = self.check_position(position).await {
                error!("Error checking position: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Check a single position for liquidation
    async fn check_position(&self, position: Position) -> Result<(), LiquidationError> {
        // Skip if position was recently liquidated
        if let Some(last_liquidated) = position.last_liquidated {
            let now = chrono::Utc::now().timestamp();
            if now - last_liquidated < self.config.liquidation_cooldown_secs {
                return Ok(());
            }
        }
        
        // Get current price from oracle
        let price = self.oracle.get_price(&position.symbol).await?;
        
        // Check if position is liquidatable
        let is_liquidatable = position.is_liquidatable(price);
        
        if is_liquidatable {
            info!("Liquidating position {} at price {}", position.address, price);
            
            // Execute liquidation
            if let Err(e) = self.liquidate_position(&position, price).await {
                error!("Failed to liquidate position {}: {}", position.address, e);
                return Err(e);
            }
            
            // Update last liquidated timestamp
            let mut positions = self.positions.write().await;
            if let Some(pos) = positions.get_mut(&position.address) {
                pos.last_liquidated = Some(chrono::Utc::now().timestamp());
            }
        }
        
        Ok(())
    }
    
    /// Execute liquidation of a position
    async fn liquidate_position(
        &self,
        position: &Position,
        price: f64,
    ) -> Result<(), LiquidationError> {
        // TODO: Implement actual liquidation logic
        // 1. Build liquidation transaction
        // 2. Sign and send transaction
        // 3. Wait for confirmation
        // 4. Handle failures and retries
        
        info!(
            "Liquidating position {}: {} {} @ {} (current price: {})",
            position.address, position.size, position.symbol, position.entry_price, price
        );
        
        // Simulate transaction processing
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        info!("Successfully liquidated position {}", position.address);
        
        Ok(())
    }
    
    /// Add a position to be monitored
    pub async fn add_position(&self, position: Position) {
        let mut positions = self.positions.write().await;
        positions.insert(position.address, position);
    }
    
    /// Remove a position from monitoring
    pub async fn remove_position(&self, address: &Pubkey) {
        let mut positions = self.positions.write().await;
        positions.remove(address);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use solana_sdk::signature::Keypair;
    use std::str::FromStr;
    
    #[tokio::test]
    async fn test_liquidation_flow() {
        // Setup mock oracle
        let mut oracle = MockOracleProvider::new();
        oracle.expect_get_price()
            .with(eq("BTC/USD"))
            .times(1)
            .returning(|_| Ok(50000.0));
        
        // Setup test position
        let position = Position {
            address: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            symbol: "BTC/USD".to_string(),
            size: 1.0,
            entry_price: 60000.0,
            margin: 0.1, // 10x leverage
            is_long: true,
            last_liquidated: None,
        };
        
        // Create engine with mock RPC client
        let rpc_client = Arc::new(RpcClient::new("https://api.devnet.solana.com".to_string()));
        let oracle = Arc::new(oracle);
        let engine = LiquidationEngine::new(rpc_client, oracle, None);
        
        // Add position and check it
        engine.add_position(position).await;
        engine.check_positions().await.unwrap();
        
        // Verify position was liquidated (check logs)
    }
}
