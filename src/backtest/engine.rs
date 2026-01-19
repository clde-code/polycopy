use crate::backtest::metrics::PerformanceMetrics;
use crate::backtest::simulator::TradeSimulator;
use crate::backtest::slippage::SlippageModel;
use crate::config::{BacktestConfig, PositionSizingConfig};
use crate::errors::{PolymarketError, Result};
use crate::execution::PositionSizer;
use crate::models::{BacktestResults, HistoricalTrade};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::info;

pub struct BacktestEngine {
    config: BacktestConfig,
    position_sizing_config: PositionSizingConfig,
    market_data: Vec<HistoricalTrade>,
    simulator: TradeSimulator,
    position_sizer: PositionSizer,
    metrics: PerformanceMetrics,
    slippage_model: SlippageModel,
}

impl BacktestEngine {
    pub fn new(config: BacktestConfig, position_sizing_config: PositionSizingConfig) -> Self {
        let fee_rate_bps = if config.apply_fees {
            config.fee_rate_bps
        } else {
            0
        };

        let slippage_model = match config.slippage_model.as_str() {
            "linear" => SlippageModel::Linear {
                depth_coefficient: config.depth_coefficient,
            },
            "percentage" => SlippageModel::Percentage {
                rate: config.slippage_percentage,
            },
            _ => SlippageModel::default(),
        };

        Self {
            simulator: TradeSimulator::new(config.initial_balance_usdc, fee_rate_bps),
            position_sizer: PositionSizer::new(position_sizing_config.clone()),
            metrics: PerformanceMetrics::new(config.initial_balance_usdc),
            market_data: Vec::new(),
            slippage_model,
            config,
            position_sizing_config,
        }
    }

    /// Run the backtest simulation
    pub async fn run(&mut self) -> Result<BacktestResults> {
        info!("Starting backtest simulation...");

        // Load historical data
        self.load_historical_data().await?;

        info!("Loaded {} historical trades", self.market_data.len());

        // Process each historical trade
        for (idx, historical_trade) in self.market_data.clone().iter().enumerate() {
            if (idx + 1) % 100 == 0 {
                info!("Processed {}/{} trades", idx + 1, self.market_data.len());
            }

            // Calculate position size for this trade
            let current_balance = self.simulator.balance();
            let my_size = match self
                .position_sizer
                .calculate_position_size(historical_trade.size, current_balance)
            {
                Ok(size) => size,
                Err(_) => continue, // Skip if position sizing fails
            };

            // Simulate execution
            match self.simulator.simulate_execution(
                &historical_trade.market,
                historical_trade.side.clone(),
                my_size,
                historical_trade.price,
                &self.slippage_model,
            ) {
                Ok(executed_trade) => {
                    self.metrics.record_trade(executed_trade);
                }
                Err(PolymarketError::InsufficientBalance) => {
                    // Skip trades we can't afford
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        // Close all positions at end of backtest
        info!("Closing all positions...");
        let market_prices = self.get_final_market_prices();
        let closed_positions = self.simulator.close_all_positions(&market_prices)?;

        for closed in closed_positions {
            self.metrics.record_closed_position(closed);
        }

        // Generate and return results
        let results = self.metrics.generate_report();
        info!("Backtest complete!");

        Ok(results)
    }

    /// Load historical trade data
    async fn load_historical_data(&mut self) -> Result<()> {
        match self.config.data_source.as_str() {
            "polymarket_api" => {
                // Mock implementation - in production, fetch from API
                info!("Loading data from Polymarket API (mock)...");
                self.market_data = self.generate_mock_data()?;
            }
            "csv_file" => {
                info!("Loading data from CSV file: {}", self.config.data_file);
                self.market_data = self.load_from_csv(&self.config.data_file)?;
            }
            _ => {
                return Err(PolymarketError::ConfigError(format!(
                    "Unknown data source: {}",
                    self.config.data_source
                )));
            }
        }

        // Filter by date range
        let start_date = NaiveDate::parse_from_str(&self.config.start_date, "%Y-%m-%d")
            .map_err(|e| PolymarketError::ParseError(format!("Invalid start date: {}", e)))?;
        let end_date = NaiveDate::parse_from_str(&self.config.end_date, "%Y-%m-%d")
            .map_err(|e| PolymarketError::ParseError(format!("Invalid end date: {}", e)))?;

        let start_datetime: DateTime<Utc> = start_date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();
        let end_datetime: DateTime<Utc> = end_date
            .and_hms_opt(23, 59, 59)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();

        self.market_data.retain(|trade| {
            trade.timestamp >= start_datetime && trade.timestamp <= end_datetime
        });

        Ok(())
    }

    /// Load data from CSV file
    fn load_from_csv(&self, _path: &str) -> Result<Vec<HistoricalTrade>> {
        // Mock implementation - would use csv crate in production
        Ok(Vec::new())
    }

    /// Generate mock historical data for testing
    fn generate_mock_data(&self) -> Result<Vec<HistoricalTrade>> {
        use crate::models::OrderSide;
        use rust_decimal_macros::dec;

        let mut trades = Vec::new();
        let trader = "0x0000000000000000000000000000000000000000"
            .parse()
            .unwrap();

        // Generate 50 sample trades
        for i in 0..50 {
            let side = if i % 2 == 0 {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            };
            let price = dec!(0.5) + (Decimal::from(i) / Decimal::from(100));
            let size = dec!(100) + (Decimal::from(i * 10));

            trades.push(HistoricalTrade {
                market: format!("market_{}", i % 5),
                side,
                price,
                size,
                timestamp: Utc::now(),
                trader,
            });
        }

        Ok(trades)
    }

    /// Get final market prices for position closing
    fn get_final_market_prices(&self) -> HashMap<String, Decimal> {
        let mut prices = HashMap::new();

        // In production, would fetch current market prices
        // For now, use average entry prices
        for trade in &self.market_data {
            prices
                .entry(trade.market.clone())
                .or_insert(trade.price);
        }

        prices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_backtest_engine() {
        let backtest_config = BacktestConfig {
            mode: "simulation".to_string(),
            start_date: "2024-01-01".to_string(),
            end_date: "2024-12-31".to_string(),
            initial_balance_usdc: dec!(10000),
            data_source: "polymarket_api".to_string(),
            data_file: "".to_string(),
            slippage_model: "linear".to_string(),
            depth_coefficient: dec!(100000),
            slippage_percentage: dec!(0.005),
            apply_fees: false,
            fee_rate_bps: 0,
            apply_gas_costs: false,
            estimated_gas_per_trade_usd: dec!(0.1),
        };

        let position_sizing_config = PositionSizingConfig {
            max_position_size_absolute: dec!(1000),
            max_position_size_relative: dec!(0.1),
            strategy: "hybrid".to_string(),
            priority: "absolute".to_string(),
        };

        let mut engine = BacktestEngine::new(backtest_config, position_sizing_config);
        let results = engine.run().await.unwrap();

        assert!(results.total_trades > 0);
        assert_eq!(results.initial_balance, dec!(10000));
    }
}
