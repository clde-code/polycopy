use crate::config::ExecutionConfig;
use crate::errors::{PolymarketError, Result};
use crate::execution::clob_client::ClobClient;
use crate::execution::position_sizer::PositionSizer;
use crate::models::{OrderFillStatus, OrderStatus, OrderType, Trade};
use rust_decimal::Decimal;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

pub struct OrderExecutor {
    clob_client: Arc<ClobClient>,
    position_sizer: Arc<PositionSizer>,
    config: ExecutionConfig,
}

impl OrderExecutor {
    pub fn new(
        clob_client: ClobClient,
        position_sizer: PositionSizer,
        config: ExecutionConfig,
    ) -> Self {
        Self {
            clob_client: Arc::new(clob_client),
            position_sizer: Arc::new(position_sizer),
            config,
        }
    }

    /// Execute a trade based on detected trader activity
    pub async fn execute_trade(&self, trade: &Trade, current_balance: Decimal) -> Result<()> {
        // Filter trade by size
        if !self.should_copy_trade(trade) {
            info!(
                "Skipping trade {} - outside configured size limits",
                trade.id
            );
            return Ok(());
        }

        // Calculate position size
        let position_size = self
            .position_sizer
            .calculate_position_size(trade.size_usdc, current_balance)?;

        info!(
            "Executing trade {} - Market: {}, Side: {}, Size: {} USDC",
            trade.id, trade.market_id, trade.side, position_size
        );

        // Determine order type
        let order_type = match self.config.order_type.as_str() {
            "FOK" => OrderType::FOK,
            "GTC" => OrderType::GTC,
            "GTD" => OrderType::GTD,
            _ => OrderType::FOK,
        };

        // Place order with retry logic
        let mut attempts = 0;
        let max_retries = self.config.max_retries;

        while attempts < max_retries {
            match self
                .clob_client
                .place_order(
                    &trade.market_id,
                    trade.side.clone(),
                    trade.price,
                    position_size,
                    order_type.clone(),
                )
                .await
            {
                Ok(order_response) => {
                    info!("Order placed successfully: {}", order_response.order_id);

                    // Monitor order fill status
                    let fill_status = self
                        .wait_for_fill(&order_response.order_id, position_size)
                        .await?;

                    match fill_status {
                        OrderFillStatus::FullyFilled { price, size } => {
                            info!(
                                "Order fully filled - Price: {}, Size: {}",
                                price, size
                            );
                            return Ok(());
                        }
                        OrderFillStatus::PartiallyFilled { price, size } => {
                            warn!(
                                "Order partially filled - Price: {}, Size: {} (expected {})",
                                price, size, position_size
                            );
                            return Ok(());
                        }
                        OrderFillStatus::TimedOut => {
                            warn!("Order timed out: {}", order_response.order_id);
                            return Err(PolymarketError::OrderTimeout);
                        }
                        OrderFillStatus::Cancelled => {
                            warn!("Order cancelled: {}", order_response.order_id);
                            return Err(PolymarketError::ExecutionError(
                                "Order was cancelled".to_string(),
                            ));
                        }
                    }
                }
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_retries {
                        error!("Failed to place order after {} attempts: {}", max_retries, e);
                        return Err(e);
                    }
                    warn!("Order placement failed (attempt {}/{}): {}", attempts, max_retries, e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }

        Err(PolymarketError::ExecutionError(
            "Max retries exceeded".to_string(),
        ))
    }

    /// Wait for an order to be filled
    async fn wait_for_fill(
        &self,
        order_id: &str,
        expected_size: Decimal,
    ) -> Result<OrderFillStatus> {
        let start = Instant::now();
        let timeout = Duration::from_millis(self.config.order_confirmation_timeout_ms);
        let poll_interval = Duration::from_millis(self.config.order_poll_interval_ms);

        loop {
            let order = self.clob_client.get_order(order_id).await?;

            match order.status {
                OrderStatus::Filled => {
                    return Ok(OrderFillStatus::FullyFilled {
                        price: Decimal::ZERO, // Would be populated from actual response
                        size: expected_size,
                    });
                }
                OrderStatus::PartiallyFilled => {
                    if start.elapsed() > timeout {
                        return Ok(OrderFillStatus::PartiallyFilled {
                            price: Decimal::ZERO,
                            size: expected_size / Decimal::from(2), // Placeholder
                        });
                    }
                }
                OrderStatus::Open => {
                    if start.elapsed() > timeout {
                        // Cancel unfilled orders
                        self.clob_client.cancel_order(order_id).await?;
                        return Ok(OrderFillStatus::TimedOut);
                    }
                }
                OrderStatus::Cancelled => {
                    return Ok(OrderFillStatus::Cancelled);
                }
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Check if a trade should be copied based on filters
    fn should_copy_trade(&self, trade: &Trade) -> bool {
        // Size filters
        if trade.size_usdc < self.config.min_trade_size_usdc {
            return false;
        }
        if trade.size_usdc > self.config.max_trade_size_usdc {
            return false;
        }

        true
    }

    /// Get current balance from CLOB client
    pub async fn get_balance(&self) -> Result<Decimal> {
        self.clob_client.get_balance().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PositionSizingConfig;
    use crate::execution::signer::OrderSigner;
    use crate::models::OrderSide;
    use chrono::Utc;
    use rust_decimal_macros::dec;

    #[test]
    fn test_should_copy_trade() {
        let config = ExecutionConfig {
            order_type: "FOK".to_string(),
            gtd_duration_seconds: 300,
            order_confirmation_timeout_ms: 30000,
            order_poll_interval_ms: 500,
            max_retries: 3,
            min_trade_size_usdc: dec!(5),
            max_trade_size_usdc: dec!(50000),
            poll_interval_seconds: 2,
        };

        let signer = OrderSigner::new(
            "0x0123456789012345678901234567890123456789012345678901234567890123",
            137,
        )
        .unwrap();
        let clob_client = ClobClient::new("http://localhost".to_string(), signer);

        let position_sizing_config = PositionSizingConfig {
            max_position_size_absolute: dec!(1000),
            max_position_size_relative: dec!(0.1),
            strategy: "hybrid".to_string(),
            priority: "absolute".to_string(),
        };
        let position_sizer = PositionSizer::new(position_sizing_config);

        let executor = OrderExecutor::new(clob_client, position_sizer, config);

        // Trade within limits
        let trade = Trade {
            id: "test".to_string(),
            market_id: "market1".to_string(),
            trader: "0x0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
            side: OrderSide::Buy,
            price: dec!(0.5),
            size: dec!(100),
            size_usdc: dec!(50),
            timestamp: Utc::now(),
            trader_win_rate: None,
        };
        assert!(executor.should_copy_trade(&trade));

        // Trade too small
        let small_trade = Trade {
            size_usdc: dec!(1),
            ..trade.clone()
        };
        assert!(!executor.should_copy_trade(&small_trade));

        // Trade too large
        let large_trade = Trade {
            size_usdc: dec!(100000),
            ..trade.clone()
        };
        assert!(!executor.should_copy_trade(&large_trade));
    }
}
