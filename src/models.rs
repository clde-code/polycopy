use chrono::{DateTime, Utc};
use ethers::types::Address;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Order side (Buy or Sell)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderSide {
    Buy,
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "BUY"),
            OrderSide::Sell => write!(f, "SELL"),
        }
    }
}

/// Order type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OrderType {
    FOK,  // Fill or Kill
    GTC,  // Good Till Cancelled
    GTD,  // Good Till Date
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderType::FOK => write!(f, "FOK"),
            OrderType::GTC => write!(f, "GTC"),
            OrderType::GTD => write!(f, "GTD"),
        }
    }
}

/// Order status
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum OrderStatus {
    Open,
    Filled,
    PartiallyFilled,
    Cancelled,
}

/// Detected trade from a monitored trader
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub market_id: String,
    pub trader: Address,
    pub side: OrderSide,
    pub price: Decimal,
    pub size: Decimal,
    pub size_usdc: Decimal,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trader_win_rate: Option<Decimal>,
}

/// Order data for signing and submission
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Order {
    pub market_id: String,
    pub price_decimal: Decimal,
    pub quantity: Decimal,
    pub side: OrderSide,
    pub owner: Address,
    pub expiration_time: u64,
}

/// Order request to send to CLOB API
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderRequest {
    pub order: Order,
    pub owner: String,
    pub order_type: String,
    pub post_only: bool,
    pub fee_rate_bps: String,
    pub side: String,
    pub signature_type: u8,
    pub signature: String,
}

/// Response from order placement
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderResponse {
    pub order_id: String,
    pub status: OrderStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Order fill status
#[derive(Clone, Debug)]
pub enum OrderFillStatus {
    FullyFilled { price: Decimal, size: Decimal },
    PartiallyFilled { price: Decimal, size: Decimal },
    TimedOut,
    Cancelled,
}

/// Executed trade (after order is filled)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutedTrade {
    pub position: Position,
    pub actual_price: Decimal,
    pub slippage: Decimal,
    pub fee: Decimal,
}

/// Position in a market
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub market_id: String,
    pub entry_price: Decimal,
    pub size: Decimal,
    pub side: OrderSide,
    pub timestamp: DateTime<Utc>,
    pub pnl: Decimal,
}

/// Closed position with exit information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClosedPosition {
    pub position: Position,
    pub exit_price: Decimal,
    pub pnl: Decimal,
    pub exit_timestamp: DateTime<Utc>,
}

/// Market data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketData {
    pub market_id: String,
    pub tick_size: Decimal,
    pub min_size: Decimal,
    pub max_size: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Trader state snapshot for monitoring
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraderState {
    pub address: Address,
    pub positions: Vec<Position>,
    pub last_updated: DateTime<Utc>,
}

/// Historical trade for backtesting
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoricalTrade {
    pub market: String,
    pub side: OrderSide,
    pub price: Decimal,
    pub size: Decimal,
    pub timestamp: DateTime<Utc>,
    pub trader: Address,
}

/// Backtest results
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BacktestResults {
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate: Decimal,
    pub total_pnl: Decimal,
    pub roi: Decimal,
    pub avg_win: Decimal,
    pub avg_loss: Decimal,
    pub profit_factor: Decimal,
    pub max_drawdown: Decimal,
    pub sharpe_ratio: Decimal,
    pub initial_balance: Decimal,
    pub final_balance: Decimal,
}

impl BacktestResults {
    pub fn format_report(&self) -> String {
        format!(
            r#"
╔══════════════════════════════════════════════════════════════╗
║              BACKTEST RESULTS                                ║
╠══════════════════════════════════════════════════════════════╣
║ Total Trades:        {:>40} ║
║ Winning Trades:      {:>40} ║
║ Losing Trades:       {:>40} ║
║ Win Rate:            {:>39}% ║
╠══════════════════════════════════════════════════════════════╣
║ Initial Balance:     {:>38} USDC ║
║ Final Balance:       {:>38} USDC ║
║ Total P&L:           {:>38} USDC ║
║ ROI:                 {:>39}% ║
╠══════════════════════════════════════════════════════════════╣
║ Average Win:         {:>38} USDC ║
║ Average Loss:        {:>38} USDC ║
║ Profit Factor:       {:>40} ║
║ Max Drawdown:        {:>39}% ║
║ Sharpe Ratio:        {:>40} ║
╚══════════════════════════════════════════════════════════════╝
"#,
            self.total_trades,
            self.winning_trades,
            self.losing_trades,
            self.win_rate.round_dp(2),
            self.initial_balance.round_dp(2),
            self.final_balance.round_dp(2),
            self.total_pnl.round_dp(2),
            self.roi.round_dp(2),
            self.avg_win.round_dp(2),
            self.avg_loss.round_dp(2),
            self.profit_factor.round_dp(2),
            self.max_drawdown.round_dp(2),
            self.sharpe_ratio.round_dp(2),
        )
    }
}
