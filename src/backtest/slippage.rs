use crate::models::OrderSide;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SlippageModel {
    Linear { depth_coefficient: Decimal },
    Percentage { rate: Decimal },
    MarketImpact { impact_param: Decimal },
}

impl SlippageModel {
    /// Calculate the actual execution price including slippage
    pub fn calculate_execution_price(
        &self,
        quote_price: Decimal,
        size: Decimal,
        side: &OrderSide,
    ) -> Decimal {
        match self {
            SlippageModel::Linear { depth_coefficient } => {
                let impact = size / *depth_coefficient;
                match side {
                    OrderSide::Buy => quote_price + impact,
                    OrderSide::Sell => quote_price - impact,
                }
            }
            SlippageModel::Percentage { rate } => {
                match side {
                    OrderSide::Buy => quote_price * (Decimal::ONE + rate),
                    OrderSide::Sell => quote_price * (Decimal::ONE - rate),
                }
            }
            SlippageModel::MarketImpact { impact_param } => {
                // Logarithmic impact model
                let size_f64 = size.to_string().parse::<f64>().unwrap_or(1.0);
                let impact_value = impact_param.to_string().parse::<f64>().unwrap_or(0.001);
                let impact = Decimal::from_f64_retain(impact_value * size_f64.ln())
                    .unwrap_or(Decimal::ZERO);

                match side {
                    OrderSide::Buy => quote_price + impact,
                    OrderSide::Sell => quote_price - impact,
                }
            }
        }
    }

    /// Calculate slippage amount (difference from quote price)
    pub fn calculate_slippage(
        &self,
        quote_price: Decimal,
        size: Decimal,
        side: &OrderSide,
    ) -> Decimal {
        let execution_price = self.calculate_execution_price(quote_price, size, side);
        (execution_price - quote_price).abs()
    }
}

impl Default for SlippageModel {
    fn default() -> Self {
        SlippageModel::Linear {
            depth_coefficient: Decimal::from(100000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_linear_slippage() {
        let model = SlippageModel::Linear {
            depth_coefficient: dec!(100000),
        };

        // Buy order should increase price
        let price = model.calculate_execution_price(dec!(0.5), dec!(1000), &OrderSide::Buy);
        assert_eq!(price, dec!(0.51)); // 0.5 + (1000/100000) = 0.51

        // Sell order should decrease price
        let price = model.calculate_execution_price(dec!(0.5), dec!(1000), &OrderSide::Sell);
        assert_eq!(price, dec!(0.49)); // 0.5 - (1000/100000) = 0.49
    }

    #[test]
    fn test_percentage_slippage() {
        let model = SlippageModel::Percentage { rate: dec!(0.01) }; // 1% slippage

        let price = model.calculate_execution_price(dec!(0.5), dec!(1000), &OrderSide::Buy);
        assert_eq!(price, dec!(0.505)); // 0.5 * 1.01

        let price = model.calculate_execution_price(dec!(0.5), dec!(1000), &OrderSide::Sell);
        assert_eq!(price, dec!(0.495)); // 0.5 * 0.99
    }

    #[test]
    fn test_slippage_calculation() {
        let model = SlippageModel::Linear {
            depth_coefficient: dec!(100000),
        };

        let slippage = model.calculate_slippage(dec!(0.5), dec!(1000), &OrderSide::Buy);
        assert_eq!(slippage, dec!(0.01));
    }
}
