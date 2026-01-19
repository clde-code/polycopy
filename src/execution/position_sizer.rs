use crate::config::PositionSizingConfig;
use crate::errors::{PolymarketError, Result};
use rust_decimal::Decimal;
use std::cmp::min;

pub struct PositionSizer {
    config: PositionSizingConfig,
}

impl PositionSizer {
    pub fn new(config: PositionSizingConfig) -> Self {
        Self { config }
    }

    /// Calculate the actual position size to execute based on the target trade size and current balance
    pub fn calculate_position_size(
        &self,
        target_trade_size: Decimal,
        current_balance: Decimal,
    ) -> Result<Decimal> {
        let mut size = target_trade_size;

        match self.config.strategy.as_str() {
            "absolute" => {
                size = min(size, self.config.max_position_size_absolute);
            }
            "relative" => {
                let relative_size = current_balance * self.config.max_position_size_relative;
                size = min(size, relative_size);
            }
            "hybrid" => {
                let relative_size = current_balance * self.config.max_position_size_relative;

                // Apply both limits
                if self.config.priority == "absolute" {
                    // Absolute takes precedence as hard cap
                    size = min(size, relative_size);
                    size = min(size, self.config.max_position_size_absolute);
                } else {
                    // Relative takes precedence as hard cap
                    size = min(size, self.config.max_position_size_absolute);
                    size = min(size, relative_size);
                }
            }
            _ => {
                return Err(PolymarketError::ConfigError(format!(
                    "Unknown position sizing strategy: {}",
                    self.config.strategy
                )));
            }
        }

        // Ensure minimum size is met
        if size <= Decimal::ZERO {
            return Err(PolymarketError::BelowMinimumSize);
        }

        Ok(size)
    }

    /// Check if a trade size is within configured limits
    pub fn is_size_acceptable(&self, size: Decimal, min_size: Decimal, max_size: Decimal) -> bool {
        size >= min_size && size <= max_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_absolute_strategy() {
        let config = PositionSizingConfig {
            max_position_size_absolute: dec!(1000),
            max_position_size_relative: dec!(0.1),
            strategy: "absolute".to_string(),
            priority: "absolute".to_string(),
        };

        let sizer = PositionSizer::new(config);

        // Trade size below limit
        let size = sizer
            .calculate_position_size(dec!(500), dec!(10000))
            .unwrap();
        assert_eq!(size, dec!(500));

        // Trade size above limit
        let size = sizer
            .calculate_position_size(dec!(2000), dec!(10000))
            .unwrap();
        assert_eq!(size, dec!(1000));
    }

    #[test]
    fn test_relative_strategy() {
        let config = PositionSizingConfig {
            max_position_size_absolute: dec!(1000),
            max_position_size_relative: dec!(0.1),
            strategy: "relative".to_string(),
            priority: "relative".to_string(),
        };

        let sizer = PositionSizer::new(config);

        // 10% of 10000 = 1000
        let size = sizer
            .calculate_position_size(dec!(2000), dec!(10000))
            .unwrap();
        assert_eq!(size, dec!(1000));

        // 10% of 5000 = 500
        let size = sizer
            .calculate_position_size(dec!(2000), dec!(5000))
            .unwrap();
        assert_eq!(size, dec!(500));
    }

    #[test]
    fn test_hybrid_strategy_absolute_priority() {
        let config = PositionSizingConfig {
            max_position_size_absolute: dec!(1000),
            max_position_size_relative: dec!(0.1),
            strategy: "hybrid".to_string(),
            priority: "absolute".to_string(),
        };

        let sizer = PositionSizer::new(config);

        // Balance: 50000, 10% = 5000, but absolute cap is 1000
        let size = sizer
            .calculate_position_size(dec!(3000), dec!(50000))
            .unwrap();
        assert_eq!(size, dec!(1000));
    }

    #[test]
    fn test_is_size_acceptable() {
        let config = PositionSizingConfig {
            max_position_size_absolute: dec!(1000),
            max_position_size_relative: dec!(0.1),
            strategy: "absolute".to_string(),
            priority: "absolute".to_string(),
        };

        let sizer = PositionSizer::new(config);

        assert!(sizer.is_size_acceptable(dec!(100), dec!(10), dec!(500)));
        assert!(!sizer.is_size_acceptable(dec!(5), dec!(10), dec!(500)));
        assert!(!sizer.is_size_acceptable(dec!(1000), dec!(10), dec!(500)));
    }
}
