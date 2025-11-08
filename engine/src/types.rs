use solana_sdk::pubkey::Pubkey;
use std::fmt;

/// Represents a liquidation event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LiquidationEvent {
    /// The address of the liquidated position
    pub position: Pubkey,
    /// The liquidator's address
    pub liquidator: Pubkey,
    /// The amount liquidated (in base currency)
    pub amount: f64,
    /// The remaining position size after liquidation
    pub remaining_size: f64,
    /// The remaining margin after liquidation
    pub remaining_margin: f64,
    /// The price at which liquidation occurred
    pub liquidation_price: f64,
    /// The timestamp of the liquidation
    pub timestamp: i64,
    /// The transaction signature
    pub signature: String,
}

/// Configuration for the liquidation engine
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LiquidationConfig {
    /// How often to check positions (in milliseconds)
    pub check_interval_ms: u64,
    /// Minimum time between liquidations for the same position (in seconds)
    pub liquidation_cooldown_secs: u64,
    /// Maximum number of positions to process in one batch
    pub max_batch_size: usize,
    /// Maximum number of concurrent liquidations
    pub max_concurrent_liquidations: usize,
    /// Maximum number of retries for failed liquidations
    pub max_retries: u8,
    /// Delay between retry attempts (in milliseconds)
    pub retry_delay_ms: u64,
    /// Whether to enable partial liquidations
    pub enable_partial_liquidations: bool,
    /// Maximum percentage of position to liquidate in a single transaction (0-100)
    pub max_liquidation_percent: u8,
    /// Minimum position size to consider for liquidation (in base currency)
    pub min_position_size: f64,
    /// Maximum position size to consider for liquidation (in base currency)
    pub max_position_size: f64,
    /// Whether to enable dry run mode (no actual transactions)
    pub dry_run: bool,
    /// List of symbols to monitor (empty for all)
    pub whitelisted_symbols: Vec<String>,
    /// List of symbols to ignore
    pub blacklisted_symbols: Vec<String>,
    /// Maximum slippage allowed for liquidations (in basis points)
    pub max_slippage_bps: u16,
    /// Priority fee in microlamports per compute unit
    pub priority_fee_micro_lamports: u64,
    /// Maintenance margin ratio (e.g., 0.05 for 5%)
    pub maintenance_margin: f64,
    /// Minimum time between liquidations (in seconds)
    pub min_liquidation_interval_secs: u64,
    /// Maximum confidence interval for oracle prices
    pub max_confidence_interval: u64,
    /// Whether to use mainnet RPC endpoints
    pub use_mainnet: bool,
}

impl Default for LiquidationConfig {
    fn default() -> Self {
        Self {
            check_interval_ms: 1000,
            liquidation_cooldown_secs: 300, // 5 minutes
            max_batch_size: 100,
            max_concurrent_liquidations: 10,
            max_retries: 3,
            retry_delay_ms: 1000,
            enable_partial_liquidations: true,
            max_liquidation_percent: 50, // 50% of position
            min_position_size: 0.001,     // 0.001 BTC
            max_position_size: 1000.0,    // 1000 BTC
            dry_run: true,
            whitelisted_symbols: vec!["BTC/USD".to_string(), "ETH/USD".to_string()],
            blacklisted_symbols: vec![],
            max_slippage_bps: 50, // 0.5%
            priority_fee_micro_lamports: 1_000, // 0.000001 SOL per CU
            maintenance_margin: 0.05, // 5%
            min_liquidation_interval_secs: 300, // 5 minutes
            max_confidence_interval: 60, // 1 minute
            use_mainnet: false,
        }
    }
}

/// Liquidation result
#[derive(Debug, Clone)]
pub enum LiquidationResult {
    /// Liquidation was successful
    Success {
        /// The liquidated position
        position: Pubkey,
        /// The amount liquidated
        amount: f64,
        /// The transaction signature
        signature: String,
    },
    /// Liquidation failed
    Failure {
        /// The liquidated position
        position: Pubkey,
        /// The error that occurred
        error: String,
        /// The number of attempts made
        attempts: u8,
    },
    /// Liquidation was skipped
    Skipped {
        /// The position that was skipped
        position: Pubkey,
        /// The reason for skipping
        reason: String,
    },
}

impl fmt::Display for LiquidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success {
                position,
                amount,
                signature,
            } => write!(
                f,
                "Liquidated {} of position {} in tx: {}",
                amount, position, signature
            ),
            Self::Failure {
                position,
                error,
                attempts,
            } => write!(
                f,
                "Failed to liquidate position {} after {} attempts: {}",
                position, attempts, error
            ),
            Self::Skipped { position, reason } => {
                write!(f, "Skipped position {}: {}", position, reason)
            }
        }
    }
}

/// Position status
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PositionStatus {
    /// Position is active and healthy
    Active,
    /// Position is at risk of liquidation
    AtRisk,
    /// Position is being liquidated
    Liquidating,
    /// Position has been fully liquidated
    Liquidated,
    /// Position is closed
    Closed,
}

impl fmt::Display for PositionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::AtRisk => write!(f, "at_risk"),
            Self::Liquidating => write!(f, "liquidating"),
            Self::Liquidated => write!(f, "liquidated"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

/// Position update event
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PositionUpdate {
    /// The position's address
    pub address: Pubkey,
    /// The owner's address
    pub owner: Pubkey,
    /// The trading pair symbol
    pub symbol: String,
    /// The current size (in base currency)
    pub size: f64,
    /// The entry price
    pub entry_price: f64,
    /// The current margin (in quote currency)
    pub margin: f64,
    /// Whether the position is long
    pub is_long: bool,
    /// The current status
    pub status: PositionStatus,
    /// The current leverage
    pub leverage: f64,
    /// The liquidation price
    pub liquidation_price: f64,
    /// The current mark price
    pub mark_price: f64,
    /// The unrealized PnL
    pub unrealized_pnl: f64,
    /// The margin ratio (as a percentage)
    pub margin_ratio: f64,
    /// The maintenance margin requirement (as a percentage)
    pub maintenance_margin: f64,
    /// The timestamp of the update
    pub timestamp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::Keypair;
    
    #[test]
    fn test_liquidation_result_display() {
        let position = Keypair::new().pubkey();
        
        let success = LiquidationResult::Success {
            position,
            amount: 1.5,
            signature: "test_sig".to_string(),
        };
        assert!(success.to_string().contains("Liquidated 1.5"));
        
        let failure = LiquidationResult::Failure {
            position,
            error: "test error".to_string(),
            attempts: 3,
        };
        assert!(failure.to_string().contains("Failed to liquidate"));
        
        let skipped = LiquidationResult::Skipped {
            position,
            reason: "test reason".to_string(),
        };
        assert!(skipped.to_string().contains("Skipped position"));
    }
    
    #[test]
    fn test_position_status_display() {
        assert_eq!(PositionStatus::Active.to_string(), "active");
        assert_eq!(PositionStatus::AtRisk.to_string(), "at_risk");
        assert_eq!(PositionStatus::Liquidating.to_string(), "liquidating");
        assert_eq!(PositionStatus::Liquidated.to_string(), "liquidated");
        assert_eq!(PositionStatus::Closed.to_string(), "closed");
    }
}
