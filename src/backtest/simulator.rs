use crate::backtest::slippage::SlippageModel;
use crate::errors::{PolymarketError, Result};
use crate::models::{ClosedPosition, ExecutedTrade, OrderSide, Position};
use chrono::Utc;
use rust_decimal::Decimal;

pub struct TradeSimulator {
    balance: Decimal,
    positions: Vec<Position>,
    fee_rate_bps: u32,
}

impl TradeSimulator {
    pub fn new(initial_balance: Decimal, fee_rate_bps: u32) -> Self {
        Self {
            balance: initial_balance,
            positions: Vec::new(),
            fee_rate_bps,
        }
    }

    /// Get current balance
    pub fn balance(&self) -> Decimal {
        self.balance
    }

    /// Get all open positions
    pub fn positions(&self) -> &[Position] {
        &self.positions
    }

    /// Simulate execution of a trade with slippage and fees
    pub fn simulate_execution(
        &mut self,
        market_id: &str,
        side: OrderSide,
        size: Decimal,
        quote_price: Decimal,
        slippage_model: &SlippageModel,
    ) -> Result<ExecutedTrade> {
        // Calculate actual execution price with slippage
        let actual_price = slippage_model.calculate_execution_price(quote_price, size, &side);
        let slippage = slippage_model.calculate_slippage(quote_price, size, &side);

        // Calculate costs
        let cost = size * actual_price;

        // Apply fees
        let fee = cost * Decimal::from(self.fee_rate_bps) / Decimal::from(10000);
        let total_cost = cost + fee;

        // Check balance
        if side == OrderSide::Buy && total_cost > self.balance {
            return Err(PolymarketError::InsufficientBalance);
        }

        // Update balance
        match side {
            OrderSide::Buy => self.balance -= total_cost,
            OrderSide::Sell => self.balance += total_cost,
        }

        // Create position
        let position = Position {
            market_id: market_id.to_string(),
            entry_price: actual_price,
            size,
            side,
            timestamp: Utc::now(),
            pnl: Decimal::ZERO,
        };

        self.positions.push(position.clone());

        Ok(ExecutedTrade {
            position,
            actual_price,
            slippage,
            fee,
        })
    }

    /// Close a position at the given exit price
    pub fn close_position(
        &mut self,
        market_id: &str,
        exit_price: Decimal,
    ) -> Result<ClosedPosition> {
        let pos_idx = self
            .positions
            .iter()
            .position(|p| p.market_id == market_id)
            .ok_or_else(|| {
                PolymarketError::SimulationError(format!("Position not found: {}", market_id))
            })?;

        let position = self.positions.remove(pos_idx);

        // Calculate P&L
        let pnl = match position.side {
            OrderSide::Buy => (exit_price - position.entry_price) * position.size,
            OrderSide::Sell => (position.entry_price - exit_price) * position.size,
        };

        // Apply exit fees
        let exit_cost = position.size * exit_price;
        let exit_fee = exit_cost * Decimal::from(self.fee_rate_bps) / Decimal::from(10000);

        // Update balance with position value and fees
        self.balance += exit_cost - exit_fee;

        Ok(ClosedPosition {
            position,
            exit_price,
            pnl: pnl - exit_fee,
            exit_timestamp: Utc::now(),
        })
    }

    /// Close all open positions at market prices
    pub fn close_all_positions(
        &mut self,
        market_prices: &std::collections::HashMap<String, Decimal>,
    ) -> Result<Vec<ClosedPosition>> {
        let mut closed = Vec::new();

        while !self.positions.is_empty() {
            let position = &self.positions[0];
            let market_id = position.market_id.clone();
            let exit_price = market_prices
                .get(&market_id)
                .copied()
                .unwrap_or(position.entry_price);

            closed.push(self.close_position(&market_id, exit_price)?);
        }

        Ok(closed)
    }

    /// Get total portfolio value (balance + position value)
    pub fn total_value(&self, market_prices: &std::collections::HashMap<String, Decimal>) -> Decimal {
        let mut total = self.balance;

        for position in &self.positions {
            let current_price = market_prices
                .get(&position.market_id)
                .copied()
                .unwrap_or(position.entry_price);

            let position_value = position.size * current_price;
            total += position_value;
        }

        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_simulate_execution() {
        let mut simulator = TradeSimulator::new(dec!(10000), 0);
        let slippage_model = SlippageModel::Linear {
            depth_coefficient: dec!(100000),
        };

        let result = simulator
            .simulate_execution(
                "market1",
                OrderSide::Buy,
                dec!(1000),
                dec!(0.5),
                &slippage_model,
            )
            .unwrap();

        // Price should be 0.5 + (1000/100000) = 0.51
        assert_eq!(result.actual_price, dec!(0.51));
        assert_eq!(result.slippage, dec!(0.01));

        // Balance should be reduced by 1000 * 0.51 = 510
        assert_eq!(simulator.balance(), dec!(9490));
    }

    #[test]
    fn test_close_position() {
        let mut simulator = TradeSimulator::new(dec!(10000), 0);
        let slippage_model = SlippageModel::Linear {
            depth_coefficient: dec!(100000),
        };

        simulator
            .simulate_execution(
                "market1",
                OrderSide::Buy,
                dec!(1000),
                dec!(0.5),
                &slippage_model,
            )
            .unwrap();

        // Close at higher price - profit
        let closed = simulator.close_position("market1", dec!(0.6)).unwrap();

        // P&L = (0.6 - 0.51) * 1000 = 90
        assert_eq!(closed.pnl, dec!(90));
    }

    #[test]
    fn test_insufficient_balance() {
        let mut simulator = TradeSimulator::new(dec!(100), 0);
        let slippage_model = SlippageModel::Linear {
            depth_coefficient: dec!(100000),
        };

        let result = simulator.simulate_execution(
            "market1",
            OrderSide::Buy,
            dec!(1000),
            dec!(0.5),
            &slippage_model,
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PolymarketError::InsufficientBalance));
    }
}
