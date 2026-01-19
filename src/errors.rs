use thiserror::Error;

#[derive(Error, Debug)]
pub enum PolymarketError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Blockchain error: {0}")]
    BlockchainError(String),

    #[error("Signing error: {0}")]
    SigningError(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Position sizing error: {0}")]
    PositionSizingError(String),

    #[error("Monitoring error: {0}")]
    MonitoringError(String),

    #[error("Simulation error: {0}")]
    SimulationError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("Order timeout")]
    OrderTimeout,

    #[error("Invalid market: {0}")]
    InvalidMarket(String),

    #[error("Invalid order size")]
    InvalidOrderSize,

    #[error("Below minimum size")]
    BelowMinimumSize,

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, PolymarketError>;
