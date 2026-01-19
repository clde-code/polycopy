use crate::errors::{PolymarketError, Result};
use crate::models::{Trade, TraderState};
use ethers::types::Address;
use reqwest::Client;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};

pub struct PollingMonitor {
    api_client: Client,
    api_url: String,
    tracked_traders: Vec<Address>,
    poll_interval: Duration,
    last_state: HashMap<Address, TraderState>,
}

impl PollingMonitor {
    pub fn new(api_url: String, tracked_traders: Vec<Address>, poll_interval: Duration) -> Self {
        Self {
            api_client: Client::new(),
            api_url,
            tracked_traders,
            poll_interval,
            last_state: HashMap::new(),
        }
    }

    /// Main monitoring loop - polls trader positions at regular intervals
    pub async fn monitor_loop<F>(&mut self, mut on_trade_detected: F) -> Result<()>
    where
        F: FnMut(&Trade) -> Result<()>,
    {
        info!("Starting polling monitor for {} traders", self.tracked_traders.len());

        loop {
            for trader_addr in &self.tracked_traders.clone() {
                match self.check_trader_activity(trader_addr).await {
                    Ok(trades) => {
                        for trade in trades {
                            debug!("Detected trade from {:?}: {:?}", trader_addr, trade.id);
                            if let Err(e) = on_trade_detected(&trade) {
                                warn!("Error handling trade: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Error checking trader {:?}: {}", trader_addr, e);
                    }
                }
            }

            tokio::time::sleep(self.poll_interval).await;
        }
    }

    /// Check a single trader for new activity
    async fn check_trader_activity(&mut self, trader: &Address) -> Result<Vec<Trade>> {
        // Fetch current positions from API
        let current_state = self.fetch_trader_positions(trader).await?;

        // Compare with previous state to detect changes
        let trades = if let Some(previous_state) = self.last_state.get(trader) {
            self.detect_position_changes(previous_state, &current_state)?
        } else {
            // First time seeing this trader - no changes to report
            Vec::new()
        };

        // Update state
        self.last_state.insert(*trader, current_state);

        Ok(trades)
    }

    /// Fetch current positions for a trader from the API
    async fn fetch_trader_positions(&self, trader: &Address) -> Result<TraderState> {
        // Mock implementation - in production, this would call the Polymarket API
        // Example endpoint: GET /positions?trader={address}

        let response = self
            .api_client
            .get(&format!("{}/positions", self.api_url))
            .query(&[("trader", format!("{:?}", trader))])
            .send()
            .await;

        match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    // Parse response into TraderState
                    // For now, return empty state
                    Ok(TraderState {
                        address: *trader,
                        positions: Vec::new(),
                        last_updated: chrono::Utc::now(),
                    })
                } else {
                    Err(PolymarketError::ApiError(format!(
                        "Failed to fetch positions: {}",
                        resp.status()
                    )))
                }
            }
            Err(e) => Err(PolymarketError::NetworkError(e)),
        }
    }

    /// Detect position changes between two states
    fn detect_position_changes(
        &self,
        previous: &TraderState,
        current: &TraderState,
    ) -> Result<Vec<Trade>> {
        let mut detected_trades = Vec::new();

        // Build map of previous positions by market
        let prev_positions: HashMap<_, _> = previous
            .positions
            .iter()
            .map(|p| (p.market_id.clone(), p))
            .collect();

        // Check for new or increased positions
        for current_pos in &current.positions {
            if let Some(prev_pos) = prev_positions.get(&current_pos.market_id) {
                // Position exists - check if size increased
                if current_pos.size > prev_pos.size {
                    let size_diff = current_pos.size - prev_pos.size;
                    detected_trades.push(Trade {
                        id: uuid::Uuid::new_v4().to_string(),
                        market_id: current_pos.market_id.clone(),
                        trader: current.address,
                        side: current_pos.side.clone(),
                        price: current_pos.entry_price,
                        size: size_diff,
                        size_usdc: size_diff * current_pos.entry_price,
                        timestamp: current_pos.timestamp,
                        trader_win_rate: None,
                    });
                }
            } else {
                // New position
                detected_trades.push(Trade {
                    id: uuid::Uuid::new_v4().to_string(),
                    market_id: current_pos.market_id.clone(),
                    trader: current.address,
                    side: current_pos.side.clone(),
                    price: current_pos.entry_price,
                    size: current_pos.size,
                    size_usdc: current_pos.size * current_pos.entry_price,
                    timestamp: current_pos.timestamp,
                    trader_win_rate: None,
                });
            }
        }

        Ok(detected_trades)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{OrderSide, Position};
    use chrono::Utc;
    use rust_decimal_macros::dec;

    #[test]
    fn test_detect_position_changes() {
        let monitor = PollingMonitor::new(
            "http://localhost".to_string(),
            vec![],
            Duration::from_secs(1),
        );

        let trader_addr = "0x0000000000000000000000000000000000000000"
            .parse()
            .unwrap();

        let previous = TraderState {
            address: trader_addr,
            positions: vec![Position {
                market_id: "market1".to_string(),
                entry_price: dec!(0.5),
                size: dec!(100),
                side: OrderSide::Buy,
                timestamp: Utc::now(),
                pnl: dec!(0),
            }],
            last_updated: Utc::now(),
        };

        let current = TraderState {
            address: trader_addr,
            positions: vec![
                Position {
                    market_id: "market1".to_string(),
                    entry_price: dec!(0.5),
                    size: dec!(150), // Increased
                    side: OrderSide::Buy,
                    timestamp: Utc::now(),
                    pnl: dec!(0),
                },
                Position {
                    market_id: "market2".to_string(),
                    entry_price: dec!(0.6),
                    size: dec!(200), // New position
                    side: OrderSide::Buy,
                    timestamp: Utc::now(),
                    pnl: dec!(0),
                },
            ],
            last_updated: Utc::now(),
        };

        let trades = monitor.detect_position_changes(&previous, &current).unwrap();
        assert_eq!(trades.len(), 2); // One increased, one new
    }
}
