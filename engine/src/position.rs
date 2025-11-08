use solana_sdk::pubkey::Pubkey;
use std::fmt;

/// Represents a trading position in the perpetual futures market
#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    /// The address of the position account on-chain
    pub address: Pubkey,
    /// The owner of the position
    pub owner: Pubkey,
    /// The trading pair symbol (e.g., "BTC/USD")
    pub symbol: String,
    /// The size of the position (in base currency)
    pub size: f64,
    /// The entry price of the position
    pub entry_price: f64,
    /// The margin allocated to the position (in quote currency)
    pub margin: f64,
    /// Whether the position is long (true) or short (false)
    pub is_long: bool,
    /// Timestamp of the last liquidation (if any)
    pub last_liquidated: Option<i64>,
}

impl Position {
    /// Create a new position
    pub fn new(
        address: Pubkey,
        owner: Pubkey,
        symbol: &str,
        size: f64,
        entry_price: f64,
        margin: f64,
        is_long: bool,
    ) -> Self {
        Self {
            address,
            owner,
            symbol: symbol.to_string(),
            size,
            entry_price,
            margin,
            is_long,
            last_liquidated: None,
        }
    }

    /// Calculate the current value of the position
    pub fn value(&self, current_price: f64) -> f64 {
        self.size * current_price
    }

    /// Calculate the unrealized PnL of the position
    pub fn unrealized_pnl(&self, current_price: f64) -> f64 {
        let price_diff = if self.is_long {
            current_price - self.entry_price
        } else {
            self.entry_price - current_price
        };
        
        self.size * price_diff
    }

    /// Calculate the margin ratio (collateral / position value)
    pub fn margin_ratio(&self, current_price: f64) -> f64 {
        let position_value = self.value(current_price);
        if position_value == 0.0 {
            return 0.0;
        }
        
        (self.margin + self.unrealized_pnl(current_price)) / position_value
    }

    /// Calculate the leverage of the position
    pub fn leverage(&self, current_price: f64) -> f64 {
        let position_value = self.value(current_price);
        if position_value == 0.0 {
            return 0.0;
        }
        
        position_value / (self.margin + self.unrealized_pnl(current_price).max(0.0))
    }

    /// Check if the position is liquidatable at the given price
    pub fn is_liquidatable(&self, current_price: f64) -> bool {
        let margin_ratio = self.margin_ratio(current_price);
        let maintenance_margin = self.calculate_maintenance_margin();
        
        margin_ratio < maintenance_margin
    }
    
    /// Check if the position is undercollateralized at the given price
    pub fn is_undercollateralized(&self, current_price: f64, maintenance_margin: f64) -> bool {
        let margin_ratio = self.margin_ratio(current_price);
        margin_ratio < maintenance_margin
    }

    /// Calculate the maintenance margin requirement based on leverage
    fn calculate_maintenance_margin(&self) -> f64 {
        // This is a simplified version - in production, this would consider
        // position size, market volatility, and other risk parameters
        const BASE_MAINTENANCE_MARGIN: f64 = 0.005; // 0.5%
        
        // Increase maintenance margin for larger positions
        let size_factor = (self.size / 1_000_000.0).min(1.0); // Cap at 1.0
        let size_impact = 0.001 * size_factor; // Up to 0.1% additional margin
        
        BASE_MAINTENANCE_MARGIN + size_impact
    }

    /// Calculate the liquidation price of the position
    pub fn liquidation_price(&self) -> f64 {
        if self.size == 0.0 {
            return 0.0;
        }

        let maintenance_margin = self.calculate_maintenance_margin();
        
        if self.is_long {
            // For long: liquidation_price = entry_price * (1 - 1/leverage + maintenance_margin)
            let leverage = self.leverage(self.entry_price);
            self.entry_price * (1.0 - 1.0 / leverage + maintenance_margin)
        } else {
            // For short: liquidation_price = entry_price * (1 + 1/leverage - maintenance_margin)
            let leverage = self.leverage(self.entry_price);
            self.entry_price * (1.0 + 1.0 / leverage - maintenance_margin)
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Position {{ address: {}, owner: {}, symbol: {}, size: {}, entry_price: ${:.2}, margin: ${:.2}, is_long: {} }}",
            self.address, self.owner, self.symbol, self.size, self.entry_price, self.margin, self.is_long
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::Keypair;
    
    fn create_test_position() -> Position {
        let owner = Keypair::new().pubkey();
        Position::new(
            Keypair::new().pubkey(),
            owner,
            "BTC/USD",
            1.0,     // 1 BTC
            60000.0, // $60,000 entry
            6000.0,  // $6,000 margin (10x leverage)
            true,    // Long position
        )
    }
    
    #[test]
    fn test_position_value() {
        let position = create_test_position();
        assert_eq!(position.value(60000.0), 60000.0);
        assert_eq!(position.value(65000.0), 65000.0);
    }
    
    #[test]
    fn test_unrealized_pnl() {
        let position = create_test_position();
        
        // Price increase (long position should be profitable)
        assert_eq!(position.unrealized_pnl(65000.0), 5000.0);
        
        // Price decrease (long position should have loss)
        assert_eq!(position.unrealized_pnl(55000.0), -5000.0);
    }
    
    #[test]
    fn test_margin_ratio() {
        let position = create_test_position();
        
        // At entry price
        let margin_ratio = position.margin_ratio(60000.0);
        assert!((margin_ratio - 0.1).abs() < 0.0001); // 10% margin ratio (6000/60000)
        
        // Price increase
        let margin_ratio = position.margin_ratio(65000.0);
        assert!(margin_ratio > 0.1);
        
        // Price decrease
        let margin_ratio = position.margin_ratio(55000.0);
        assert!(margin_ratio < 0.1);
    }
    
    #[test]
    fn test_leverage() {
        let position = create_test_position();
        
        // At entry price, leverage should be 10x
        let leverage = position.leverage(60000.0);
        assert!((leverage - 10.0).abs() < 0.01);
        
        // With profit, leverage decreases
        let leverage = position.leverage(65000.0);
        assert!(leverage < 10.0);
        
        // With loss, leverage increases
        let leverage = position.leverage(55000.0);
        assert!(leverage > 10.0);
    }
    
    #[test]
    fn test_liquidation_price() {
        let position = create_test_position();
        let liq_price = position.liquidation_price();
        
        // For a 10x long position with ~0.5% maintenance, liquidation should be around 5-6% below entry
        assert!(liq_price < position.entry_price * 0.95);
        assert!(liq_price > position.entry_price * 0.90);
    }
    
    #[test]
    fn test_is_liquidatable() {
        let position = create_test_position();
        
        // At entry price, should not be liquidatable
        assert!(!position.is_liquidatable(60000.0));
        
        // At liquidation price, should be liquidatable
        let liq_price = position.liquidation_price();
        assert!(position.is_liquidatable(liq_price));
        
        // Below liquidation price, should be liquidatable
        assert!(position.is_liquidatable(liq_price * 0.9));
    }
}
