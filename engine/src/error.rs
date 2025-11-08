use solana_client::client_error::ClientError;
use solana_sdk::{program_error::ProgramError, pubkey::Pubkey};
use std::fmt;

/// Custom error type for the liquidation engine
#[derive(Debug)]
pub enum LiquidationError {
    /// Error from Solana RPC client
    RpcError(String),
    
    /// Error from Solana program
    ProgramError(ProgramError),
    
    /// Error from oracle service
    OracleError(String),
    
    /// Price is stale (older than allowed threshold)
    StalePrice(String),
    
    /// Price confidence is too low
    LowConfidencePrice(String),
    
    /// Price confidence interval is too wide
    HighConfidenceInterval(String),
    
    /// Position is not liquidatable
    PositionNotLiquidatable(Pubkey),
    
    /// Liquidation failed
    LiquidationFailed(String),
    
    /// Transaction simulation failed
    SimulationFailed(String),
    
    /// Transaction confirmation timeout
    ConfirmationTimeout,
    
    /// Invalid configuration
    ConfigError(String),
    
    /// Other errors
    Other(String),
}

impl fmt::Display for LiquidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RpcError(msg) => write!(f, "RPC error: {}", msg),
            Self::ProgramError(err) => write!(f, "Program error: {}", err),
            Self::OracleError(msg) => write!(f, "Oracle error: {}", msg),
            Self::StalePrice(symbol) => write!(f, "Stale price for {}", symbol),
            Self::LowConfidencePrice(symbol) => write!(f, "Low confidence price for {}", symbol),
            Self::HighConfidenceInterval(symbol) => write!(f, "High confidence interval for {}", symbol),
            Self::PositionNotLiquidatable(address) => write!(f, "Position {} is not liquidatable", address),
            Self::LiquidationFailed(msg) => write!(f, "Liquidation failed: {}", msg),
            Self::SimulationFailed(msg) => write!(f, "Simulation failed: {}", msg),
            Self::ConfirmationTimeout => write!(f, "Transaction confirmation timed out"),
            Self::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            Self::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for LiquidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::RpcError(_) => None,
            Self::ProgramError(e) => Some(e),
            Self::OracleError(_) => None,
            Self::StalePrice(_) => None,
            Self::LowConfidencePrice(_) => None,
            Self::HighConfidenceInterval(_) => None,
            Self::PositionNotLiquidatable(_) => None,
            Self::LiquidationFailed(_) => None,
            Self::SimulationFailed(_) => None,
            Self::ConfirmationTimeout => None,
            Self::ConfigError(_) => None,
            Self::Other(_) => None,
        }
    }
}

impl From<Box<dyn std::error::Error>> for LiquidationError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        Self::Other(err.to_string())
    }
}

impl From<String> for LiquidationError {
    fn from(err: String) -> Self {
        Self::Other(err)
    }
}

impl From<ClientError> for LiquidationError {
    fn from(err: ClientError) -> Self {
        Self::RpcError(err.to_string())
    }
}

impl From<ProgramError> for LiquidationError {
    fn from(err: ProgramError) -> Self {
        Self::ProgramError(err)
    }
}

impl From<std::io::Error> for LiquidationError {
    fn from(err: std::io::Error) -> Self {
        Self::Other(err.to_string())
    }
}

impl From<serde_json::Error> for LiquidationError {
    fn from(err: serde_json::Error) -> Self {
        Self::Other(err.to_string())
    }
}

impl From<solana_sdk::signature::SignerError> for LiquidationError {
    fn from(err: solana_sdk::signature::SignerError) -> Self {
        Self::Other(err.to_string())
    }
}

impl From<solana_sdk::transaction::TransactionError> for LiquidationError {
    fn from(err: solana_sdk::transaction::TransactionError) -> Self {
        Self::Other(err.to_string())
    }
}

impl From<solana_sdk::pubkey::ParsePubkeyError> for LiquidationError {
    fn from(err: solana_sdk::pubkey::ParsePubkeyError) -> Self {
        Self::ConfigError(err.to_string())
    }
}

impl From<std::num::ParseIntError> for LiquidationError {
    fn from(err: std::num::ParseIntError) -> Self {
        Self::ConfigError(err.to_string())
    }
}

impl From<std::string::FromUtf8Error> for LiquidationError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Self::Other(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for LiquidationError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        Self::ConfirmationTimeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_display() {
        let error = LiquidationError::RpcError("Connection failed".to_string());
        assert_eq!(error.to_string(), "RPC error: Connection failed");
        
        let error = LiquidationError::StalePrice("BTC/USD".to_string());
        assert_eq!(error.to_string(), "Stale price for BTC/USD");
        
        let error = LiquidationError::Other("Something went wrong".to_string());
        assert_eq!(error.to_string(), "Error: Something went wrong");
    }
    
    #[test]
    fn test_error_conversions() {
        let io_error = std::io::Error::new(std::io::ErrorKind::Other, "IO error");
        let error: LiquidationError = io_error.into();
        assert!(matches!(error, LiquidationError::Other(_)));
        
        let parse_int_error = "not a number".parse::<i32>().unwrap_err();
        let error: LiquidationError = parse_int_error.into();
        assert!(matches!(error, LiquidationError::ConfigError(_)));
    }
}
