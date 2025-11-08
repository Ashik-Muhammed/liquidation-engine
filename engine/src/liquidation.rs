use crate::{
    error::LiquidationError,
    oracle::OracleProvider,
    position::Position,
    types::LiquidationConfig,
};
use anchor_lang::prelude::*;
use log::{error, info};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Duration;
use std::result::Result as StdResult;

/// Main LiquidationEngine that monitors and liquidates undercollateralized positions
pub struct LiquidationEngine {
    /// RPC client for Solana
    rpc_client: Arc<RpcClient>,
    /// Oracle for price feeds
    oracle: Arc<dyn OracleProvider + Send + Sync>,
    /// Configuration parameters
    config: LiquidationConfig,
    /// Cache of monitored positions
    positions: RwLock<HashMap<Pubkey, Position>>,
}

impl LiquidationEngine {
    /// Create a new LiquidationEngine instance
    pub fn new(
        rpc_client: Arc<RpcClient>,
        oracle: Arc<dyn OracleProvider + Send + Sync>,
        config: LiquidationConfig,
    ) -> Self {
        Self {
            rpc_client,
            oracle,
            config,
            positions: RwLock::new(HashMap::new()),
        }
    }

    /// Start the liquidation monitoring service
    pub async fn start(&self) -> StdResult<(), LiquidationError> {
        info!("Starting liquidation engine");
        let mut interval = tokio::time::interval(Duration::from_millis(self.config.check_interval_ms));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.check_positions().await {
                error!("Error checking positions: {}", e);
                continue;
            }
        }
    }
    
    /// Check all monitored positions for liquidation
    pub async fn check_positions(&self) -> StdResult<(), LiquidationError> {
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
    async fn check_position(&self, position: Position) -> StdResult<(), LiquidationError> {
        // Skip if position was recently liquidated
        if let Some(last_liquidated) = position.last_liquidated {
            let now = chrono::Utc::now().timestamp() as u64;
            if now.saturating_sub(last_liquidated as u64) < self.config.min_liquidation_interval_secs {
                return Ok(());
            }
        }
        
        // Get the current price from the oracle
        let price = self.oracle.get_price(&position.symbol).await?;
        
        // Check if the position is undercollateralized
        if position.is_undercollateralized(price, self.config.maintenance_margin) {
            info!("Liquidating position: {:?} at price: {}", position, price);
            self.liquidate_position(&position, price).await?;
        }
        
        Ok(())
    }
    
    /// Execute liquidation of a position
    async fn liquidate_position(
        &self,
        position: &Position,
        price: f64,
    ) -> StdResult<(), LiquidationError> {
        // Implement liquidation logic here
        // This would involve:
        // 1. Creating and sending a transaction to the Solana network
        // 2. Updating the position's state
        // 3. Emitting events/logs
        
        info!("Liquidating position: {:?} at price: {}", position, price);
        
        // In a real implementation, we would:
        // 1. Create a transaction to liquidate the position
        // 2. Sign and send the transaction
        // 3. Update the position's state
        
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
    
    /// Get a reference to the engine's configuration
    pub fn config(&self) -> &LiquidationConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::Keypair;
    use std::str::FromStr;
    
    #[test]
    fn test_engine_initialization() {
        let rpc_client = Arc::new(RpcClient::new("https://api.devnet.solana.com"));
        let oracle = Arc::new(PythOracle::new(
            "https://api.devnet.solana.com",
            HashMap::new(),
            None,
        ));
        
        let config = LiquidationConfig::default();
        let engine = LiquidationEngine::new(rpc_client, oracle, config);
        
        assert_eq!(engine.positions.blocking_read().len(), 0);
    }
}
