use crate::errors::{PolymarketError, Result};
use ethers::types::Address;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub traders: TradersConfig,
    pub position_sizing: PositionSizingConfig,
    pub execution: ExecutionConfig,
    pub backtest: BacktestConfig,
    pub logging: LoggingConfig,
    pub database: DatabaseConfig,
    #[serde(default)]
    pub notifications: NotificationsConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub mode: String,
    pub wallet_private_key: String,
    pub polygon_rpc_url: String,
    pub polymarket_api_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradersConfig {
    pub tracked_accounts: Vec<String>,
}

impl TradersConfig {
    pub fn get_addresses(&self) -> Result<Vec<Address>> {
        self.tracked_accounts
            .iter()
            .map(|addr| {
                addr.parse::<Address>()
                    .map_err(|e| PolymarketError::ParseError(format!("Invalid address {}: {}", addr, e)))
            })
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionSizingConfig {
    pub max_position_size_absolute: Decimal,
    pub max_position_size_relative: Decimal,
    pub strategy: String, // "absolute", "relative", or "hybrid"
    pub priority: String, // "absolute" or "relative"
}

impl PositionSizingConfig {
    pub fn is_valid(&self) -> bool {
        matches!(self.strategy.as_str(), "absolute" | "relative" | "hybrid")
            && matches!(self.priority.as_str(), "absolute" | "relative")
            && self.max_position_size_absolute > Decimal::ZERO
            && self.max_position_size_relative > Decimal::ZERO
            && self.max_position_size_relative <= Decimal::ONE
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub order_type: String,
    pub gtd_duration_seconds: u64,
    pub order_confirmation_timeout_ms: u64,
    pub order_poll_interval_ms: u64,
    pub max_retries: u32,
    pub min_trade_size_usdc: Decimal,
    pub max_trade_size_usdc: Decimal,
    pub poll_interval_seconds: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub mode: String,
    pub start_date: String,
    pub end_date: String,
    pub initial_balance_usdc: Decimal,
    pub data_source: String,
    pub data_file: String,
    pub slippage_model: String,
    pub depth_coefficient: Decimal,
    pub slippage_percentage: Decimal,
    pub apply_fees: bool,
    pub fee_rate_bps: u32,
    pub apply_gas_costs: bool,
    pub estimated_gas_per_trade_usd: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_output: String,
    pub max_log_size_mb: u64,
    pub log_retention_days: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub db_type: String,
    pub db_connection: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NotificationsConfig {
    pub slack_webhook_url: Option<String>,
    pub notify_on_trade: bool,
    pub notify_on_error: bool,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|e| {
            PolymarketError::ConfigError(format!("Failed to read config file: {}", e))
        })?;

        let config: Config = toml::from_str(&content)?;
        config.validate()?;

        Ok(config)
    }

    /// Validate configuration
    fn validate(&self) -> Result<()> {
        // Validate mode
        if !matches!(self.general.mode.as_str(), "live" | "backtest") {
            return Err(PolymarketError::ConfigError(
                "Invalid mode. Must be 'live' or 'backtest'".to_string(),
            ));
        }

        // Validate position sizing
        if !self.position_sizing.is_valid() {
            return Err(PolymarketError::ConfigError(
                "Invalid position sizing configuration".to_string(),
            ));
        }

        // Validate trader addresses
        let _ = self.traders.get_addresses()?;

        // Validate execution config
        if self.execution.min_trade_size_usdc >= self.execution.max_trade_size_usdc {
            return Err(PolymarketError::ConfigError(
                "min_trade_size_usdc must be less than max_trade_size_usdc".to_string(),
            ));
        }

        Ok(())
    }

    /// Expand environment variables in configuration
    pub fn expand_env_vars(&mut self) -> Result<()> {
        // Expand wallet private key
        if self.general.wallet_private_key.starts_with("${") && self.general.wallet_private_key.ends_with("}") {
            let var_name = &self.general.wallet_private_key[2..self.general.wallet_private_key.len() - 1];
            self.general.wallet_private_key = std::env::var(var_name).map_err(|_| {
                PolymarketError::ConfigError(format!(
                    "Environment variable {} not set",
                    var_name
                ))
            })?;
        }

        // Expand slack webhook URL if present
        if let Some(ref webhook) = self.notifications.slack_webhook_url {
            if webhook.starts_with("${") && webhook.ends_with("}") {
                let var_name = &webhook[2..webhook.len() - 1];
                self.notifications.slack_webhook_url = std::env::var(var_name).ok();
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_sizing_validation() {
        let valid_config = PositionSizingConfig {
            max_position_size_absolute: Decimal::from(1000),
            max_position_size_relative: Decimal::new(1, 1), // 0.1
            strategy: "hybrid".to_string(),
            priority: "absolute".to_string(),
        };
        assert!(valid_config.is_valid());

        let invalid_config = PositionSizingConfig {
            max_position_size_absolute: Decimal::from(1000),
            max_position_size_relative: Decimal::new(15, 1), // 1.5 > 1.0
            strategy: "hybrid".to_string(),
            priority: "absolute".to_string(),
        };
        assert!(!invalid_config.is_valid());
    }
}
