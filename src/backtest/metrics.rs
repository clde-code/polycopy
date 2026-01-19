use crate::models::{BacktestResults, ClosedPosition, ExecutedTrade};
use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;

pub struct PerformanceMetrics {
    trades: Vec<ExecutedTrade>,
    closed_positions: Vec<ClosedPosition>,
    initial_balance: Decimal,
}

impl PerformanceMetrics {
    pub fn new(initial_balance: Decimal) -> Self {
        Self {
            trades: Vec::new(),
            closed_positions: Vec::new(),
            initial_balance,
        }
    }

    /// Record a trade execution
    pub fn record_trade(&mut self, trade: ExecutedTrade) {
        self.trades.push(trade);
    }

    /// Record a closed position
    pub fn record_closed_position(&mut self, position: ClosedPosition) {
        self.closed_positions.push(position);
    }

    /// Generate comprehensive backtest results
    pub fn generate_report(&self) -> BacktestResults {
        let total_pnl: Decimal = self.closed_positions.iter().map(|p| p.pnl).sum();
        let winning_trades = self
            .closed_positions
            .iter()
            .filter(|p| p.pnl > Decimal::ZERO)
            .count();
        let losing_trades = self
            .closed_positions
            .iter()
            .filter(|p| p.pnl < Decimal::ZERO)
            .count();

        let win_rate = if self.closed_positions.is_empty() {
            Decimal::ZERO
        } else {
            Decimal::from(winning_trades) / Decimal::from(self.closed_positions.len())
        };

        let avg_win = if winning_trades == 0 {
            Decimal::ZERO
        } else {
            let sum: Decimal = self
                .closed_positions
                .iter()
                .filter(|p| p.pnl > Decimal::ZERO)
                .map(|p| p.pnl)
                .sum();
            sum / Decimal::from(winning_trades)
        };

        let avg_loss = if losing_trades == 0 {
            Decimal::ZERO
        } else {
            let sum: Decimal = self
                .closed_positions
                .iter()
                .filter(|p| p.pnl < Decimal::ZERO)
                .map(|p| p.pnl.abs())
                .sum();
            sum / Decimal::from(losing_trades)
        };

        let profit_factor = if avg_loss == Decimal::ZERO {
            Decimal::from(1000) // Cap at 1000 if no losses
        } else {
            avg_win / avg_loss
        };

        let roi = if self.initial_balance > Decimal::ZERO {
            (total_pnl / self.initial_balance) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        let max_drawdown = self.calculate_max_drawdown();
        let sharpe = self.calculate_sharpe_ratio();

        BacktestResults {
            total_trades: self.closed_positions.len(),
            winning_trades,
            losing_trades,
            win_rate: win_rate * Decimal::from(100),
            total_pnl,
            roi,
            avg_win,
            avg_loss,
            profit_factor,
            max_drawdown,
            sharpe_ratio: sharpe,
            initial_balance: self.initial_balance,
            final_balance: self.initial_balance + total_pnl,
        }
    }

    /// Calculate maximum drawdown as percentage
    fn calculate_max_drawdown(&self) -> Decimal {
        let mut peak = self.initial_balance;
        let mut max_dd = Decimal::ZERO;
        let mut current_balance = self.initial_balance;

        for position in &self.closed_positions {
            current_balance += position.pnl;
            if current_balance > peak {
                peak = current_balance;
            }
            let dd = (peak - current_balance) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }

        max_dd * Decimal::from(100)
    }

    /// Calculate Sharpe ratio (simplified version)
    fn calculate_sharpe_ratio(&self) -> Decimal {
        if self.closed_positions.is_empty() {
            return Decimal::ZERO;
        }

        let returns: Vec<Decimal> = self.closed_positions.iter().map(|p| p.pnl).collect();

        let mean_return: Decimal =
            returns.iter().sum::<Decimal>() / Decimal::from(returns.len());

        // Calculate standard deviation
        let variance: Decimal = returns
            .iter()
            .map(|r| {
                let diff = *r - mean_return;
                diff * diff
            })
            .sum::<Decimal>()
            / Decimal::from(returns.len());

        let std_dev = variance.sqrt().unwrap_or(Decimal::ONE);

        if std_dev == Decimal::ZERO {
            return Decimal::ZERO;
        }

        // Sharpe ratio = mean / std_dev (assuming risk-free rate = 0)
        mean_return / std_dev
    }

    /// Get total fees paid
    pub fn total_fees(&self) -> Decimal {
        self.trades.iter().map(|t| t.fee).sum()
    }

    /// Get total slippage incurred
    pub fn total_slippage(&self) -> Decimal {
        self.trades.iter().map(|t| t.slippage * t.position.size).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{OrderSide, Position};
    use chrono::Utc;
    use rust_decimal_macros::dec;

    #[test]
    fn test_metrics_calculation() {
        let mut metrics = PerformanceMetrics::new(dec!(10000));

        // Add some closed positions
        metrics.record_closed_position(ClosedPosition {
            position: Position {
                market_id: "m1".to_string(),
                entry_price: dec!(0.5),
                size: dec!(100),
                side: OrderSide::Buy,
                timestamp: Utc::now(),
                pnl: dec!(0),
            },
            exit_price: dec!(0.6),
            pnl: dec!(10), // Win
            exit_timestamp: Utc::now(),
        });

        metrics.record_closed_position(ClosedPosition {
            position: Position {
                market_id: "m2".to_string(),
                entry_price: dec!(0.5),
                size: dec!(100),
                side: OrderSide::Buy,
                timestamp: Utc::now(),
                pnl: dec!(0),
            },
            exit_price: dec!(0.4),
            pnl: dec!(-10), // Loss
            exit_timestamp: Utc::now(),
        });

        metrics.record_closed_position(ClosedPosition {
            position: Position {
                market_id: "m3".to_string(),
                entry_price: dec!(0.5),
                size: dec!(100),
                side: OrderSide::Buy,
                timestamp: Utc::now(),
                pnl: dec!(0),
            },
            exit_price: dec!(0.7),
            pnl: dec!(20), // Win
            exit_timestamp: Utc::now(),
        });

        let results = metrics.generate_report();

        assert_eq!(results.total_trades, 3);
        assert_eq!(results.winning_trades, 2);
        assert_eq!(results.losing_trades, 1);
        assert_eq!(results.total_pnl, dec!(20)); // 10 - 10 + 20
        assert_eq!(results.avg_win, dec!(15)); // (10 + 20) / 2
        assert_eq!(results.avg_loss, dec!(10));
    }
}
