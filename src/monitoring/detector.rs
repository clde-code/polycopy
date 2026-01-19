use crate::models::Trade;
use rust_decimal::Decimal;
use std::collections::HashSet;
use std::time::Duration;

pub struct TradeFilter {
    pub min_size_usdc: Decimal,
    pub max_size_usdc: Decimal,
    pub allowed_markets: Option<HashSet<String>>,
    pub min_market_duration: Duration,
    pub filter_by_win_rate: Option<Decimal>,
}

impl TradeFilter {
    pub fn new(min_size_usdc: Decimal, max_size_usdc: Decimal) -> Self {
        Self {
            min_size_usdc,
            max_size_usdc,
            allowed_markets: None,
            min_market_duration: Duration::from_secs(3600), // 1 hour default
            filter_by_win_rate: None,
        }
    }

    pub fn with_allowed_markets(mut self, markets: HashSet<String>) -> Self {
        self.allowed_markets = Some(markets);
        self
    }

    pub fn with_min_duration(mut self, duration: Duration) -> Self {
        self.min_market_duration = duration;
        self
    }

    pub fn with_min_win_rate(mut self, win_rate: Decimal) -> Self {
        self.filter_by_win_rate = Some(win_rate);
        self
    }

    /// Check if a trade should be copied based on filters
    pub fn should_copy(&self, trade: &Trade) -> bool {
        // Size filters
        if trade.size_usdc < self.min_size_usdc {
            return false;
        }
        if trade.size_usdc > self.max_size_usdc {
            return false;
        }

        // Market filters
        if let Some(ref allowed) = self.allowed_markets {
            if !allowed.contains(&trade.market_id) {
                return false;
            }
        }

        // Trader track record
        if let Some(min_wr) = self.filter_by_win_rate {
            if let Some(trader_wr) = trade.trader_win_rate {
                if trader_wr < min_wr {
                    return false;
                }
            } else {
                // No win rate data - reject if filter is set
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::OrderSide;
    use chrono::Utc;
    use rust_decimal_macros::dec;

    #[test]
    fn test_trade_filter_size() {
        let filter = TradeFilter::new(dec!(10), dec!(1000));

        let valid_trade = Trade {
            id: "1".to_string(),
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

        assert!(filter.should_copy(&valid_trade));

        let too_small = Trade {
            size_usdc: dec!(5),
            ..valid_trade.clone()
        };
        assert!(!filter.should_copy(&too_small));

        let too_large = Trade {
            size_usdc: dec!(5000),
            ..valid_trade.clone()
        };
        assert!(!filter.should_copy(&too_large));
    }

    #[test]
    fn test_trade_filter_win_rate() {
        let filter = TradeFilter::new(dec!(10), dec!(1000)).with_min_win_rate(dec!(0.6));

        let high_wr_trade = Trade {
            id: "1".to_string(),
            market_id: "market1".to_string(),
            trader: "0x0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
            side: OrderSide::Buy,
            price: dec!(0.5),
            size: dec!(100),
            size_usdc: dec!(50),
            timestamp: Utc::now(),
            trader_win_rate: Some(dec!(0.7)),
        };

        assert!(filter.should_copy(&high_wr_trade));

        let low_wr_trade = Trade {
            trader_win_rate: Some(dec!(0.4)),
            ..high_wr_trade.clone()
        };
        assert!(!filter.should_copy(&low_wr_trade));
    }
}
