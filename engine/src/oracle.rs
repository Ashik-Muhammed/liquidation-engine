use crate::error::LiquidationError;
use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

// Newtype wrapper to implement Debug for RpcClient
#[derive(Clone)]
struct DebuggableRpcClient(Arc<solana_client::rpc_client::RpcClient>);

impl fmt::Debug for DebuggableRpcClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RpcClient").finish()
    }
}

/// Trait for price oracle providers
#[async_trait]
pub trait OracleProvider: Send + Sync + std::fmt::Debug {
    /// Get the current price for a symbol
    async fn get_price(&self, symbol: &str) -> Result<f64, LiquidationError>;
    
    /// Get multiple prices at once (for batch processing)
    async fn get_prices(&self, symbols: &[&str]) -> Result<HashMap<String, f64>, LiquidationError> {
        let mut prices = HashMap::new();
        for &symbol in symbols {
            let price = self.get_price(symbol).await?;
            prices.insert(symbol.to_string(), price);
        }
        Ok(prices)
    }
    
    /// Get the last update time for a price feed
    async fn last_update_time(&self, _symbol: &str) -> Result<u64, LiquidationError> {
        // Default implementation returns current timestamp
        Ok(chrono::Utc::now().timestamp() as u64)
    }
}

/// Pyth Network Oracle implementation
#[derive(Debug, Clone)]
pub struct PythOracle {
    /// RPC client for Solana
    rpc_client: DebuggableRpcClient,
    /// Cache of price accounts
    price_accounts: Arc<RwLock<HashMap<String, Pubkey>>>,
    /// Price feed configuration
    config: OracleConfig,
}

/// Oracle configuration
#[derive(Debug, Clone)]
pub struct OracleConfig {
    /// Maximum allowed price age in seconds
    pub max_price_age_secs: u64,
    /// Minimum confidence interval (as a percentage of price)
    pub min_confidence_interval: f64,
    /// Maximum confidence interval (as a percentage of price)
    pub max_confidence_interval: f64,
    /// Whether to use the Pyth mainnet program
    pub use_mainnet: bool,
}

impl Default for OracleConfig {
    fn default() -> Self {
        Self {
            max_price_age_secs: 30, // 30 seconds
            min_confidence_interval: 0.001, // 0.1%
            max_confidence_interval: 0.01,  // 1%
            use_mainnet: false,
        }
    }
}

impl PythOracle {
    /// Create a new PythOracle instance
    pub fn new(
        rpc_url: &str,
        price_accounts: HashMap<String, Pubkey>,
        config: Option<OracleConfig>,
    ) -> Self {
        let rpc_client = DebuggableRpcClient(Arc::new(
            solana_client::rpc_client::RpcClient::new(rpc_url.to_string()),
        ));
        Self {
            rpc_client,
            price_accounts: Arc::new(RwLock::new(price_accounts)),
            config: config.unwrap_or_default(),
        }
    }
    
    /// Add or update a price account
    pub async fn add_price_account(&self, symbol: &str, pubkey: Pubkey) {
        let mut accounts = self.price_accounts.write().await;
        accounts.insert(symbol.to_string(), pubkey);
    }
    
    /// Get the price account for a symbol
    pub async fn get_price_account(&self, symbol: &str) -> Option<Pubkey> {
        let accounts = self.price_accounts.read().await;
        accounts.get(symbol).copied()
    }
    
    fn get_rpc_client(&self) -> Arc<solana_client::rpc_client::RpcClient> {
        self.rpc_client.0.clone()
    }
}

#[async_trait]
impl OracleProvider for PythOracle {
    async fn get_price(&self, symbol: &str) -> Result<f64, LiquidationError> {
        // Get the price account for the symbol
        let price_account = self
            .get_price_account(symbol)
            .await
            .ok_or_else(|| LiquidationError::OracleError(format!("No price account for {}", symbol)))?;
            
        // Fetch the price account data
        let account_data = self
            .get_rpc_client()
            .get_account_data(&price_account)
            .map_err(|e| LiquidationError::RpcError(e.to_string()))?;
            
        // Parse the price data using Pyth's SDK
        let price_account = pyth_sdk_solana::state::load_price_account(&account_data)
            .map_err(|e| LiquidationError::OracleError(e.to_string()))?;
            
        // Check if the price is stale
        let last_update_time = price_account.timestamp;
        let current_time = chrono::Utc::now().timestamp() as u64;
        
        if current_time.saturating_sub(last_update_time as u64) > self.config.max_price_age_secs {
            return Err(LiquidationError::StalePrice(symbol.to_string()));
        }
        
        // Get the current price and confidence interval
        let price = price_account.agg.price as f64 * 10f64.powi(price_account.expo as i32);
        let confidence = price_account.agg.conf as f64 * 10f64.powi(price_account.expo as i32);
        
        // Check confidence interval
        let confidence_ratio = confidence / price;
        if confidence_ratio < self.config.min_confidence_interval {
            return Err(LiquidationError::LowConfidencePrice(symbol.to_string()));
        }
        
        if confidence_ratio > self.config.max_confidence_interval {
            return Err(LiquidationError::HighConfidenceInterval(symbol.to_string()));
        }
        
        Ok(price)
    }
}

/// Mock oracle for testing
#[derive(Debug, Clone, Default)]
pub struct MockOracle {
    prices: Arc<RwLock<HashMap<String, f64>>>,
}

impl MockOracle {
    /// Create a new mock oracle
    pub fn new() -> Self {
        Self {
            prices: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Set a price for a symbol
    pub async fn set_price(&self, symbol: &str, price: f64) {
        let mut prices = self.prices.write().await;
        prices.insert(symbol.to_string(), price);
    }
}

#[async_trait]
impl OracleProvider for MockOracle {
    async fn get_price(&self, symbol: &str) -> Result<f64, LiquidationError> {
        self.prices
            .read()
            .await
            .get(symbol)
            .copied()
            .ok_or_else(|| LiquidationError::OracleError(format!("No price for {}", symbol)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::Keypair;
    
    #[tokio::test]
    async fn test_mock_oracle() {
        let oracle = MockOracle::new();
        oracle.set_price("BTC/USD", 50000.0).await;
        
        let price = oracle.get_price("BTC/USD").await.unwrap();
        assert_eq!(price, 50000.0);
        
        // Test non-existent symbol
        assert!(oracle.get_price("NON_EXISTENT").await.is_err());
    }
    
    // Note: PythOracle tests would require a running Solana validator
    // with Pyth price accounts, which is beyond the scope of unit tests
}
