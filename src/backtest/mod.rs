pub mod engine;
pub mod metrics;
pub mod simulator;
pub mod slippage;

pub use engine::BacktestEngine;
pub use metrics::PerformanceMetrics;
pub use simulator::TradeSimulator;
pub use slippage::SlippageModel;
