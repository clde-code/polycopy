mod backtest;
mod config;
mod errors;
mod execution;
mod models;
mod monitoring;
mod storage;

use backtest::BacktestEngine;
use clap::Parser;
use config::Config;
use errors::Result;
use execution::{ClobClient, OrderExecutor, OrderSigner, PositionSizer};
use monitoring::PollingMonitor;
use storage::TradeLogger;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(name = "Polymarket Copy Trader")]
#[command(author = "Polymarket Copy Trader Contributors")]
#[command(version = "0.1.0")]
#[command(about = "A self-hosted Rust-based copy-trading bot for Polymarket", long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// Operating mode (overrides config): live or backtest
    #[arg(short, long)]
    mode: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Parse command line arguments
    let args = Args::parse();

    // Load configuration
    let mut config = Config::load_from_file(&args.config)?;
    config.expand_env_vars()?;

    // Override mode if specified in CLI
    if let Some(mode) = args.mode {
        config.general.mode = mode;
    }

    // Initialize logging
    init_logging(&config.logging.level)?;

    info!("Starting Polymarket Copy Trader v0.1.0");
    info!("Mode: {}", config.general.mode);

    // Run appropriate mode
    match config.general.mode.as_str() {
        "live" => run_live_trading(config).await,
        "backtest" => run_backtest(config).await,
        _ => {
            error!("Invalid mode: {}", config.general.mode);
            Err(errors::PolymarketError::ConfigError(format!(
                "Invalid mode: {}. Must be 'live' or 'backtest'",
                config.general.mode
            )))
        }
    }
}

/// Initialize logging based on configuration
fn init_logging(level: &str) -> Result<()> {
    let log_level = match level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| errors::PolymarketError::Unknown(format!("Failed to set logger: {}", e)))?;

    Ok(())
}

/// Run live copy trading
async fn run_live_trading(config: Config) -> Result<()> {
    info!("Initializing live trading mode...");

    // Initialize components
    let signer = OrderSigner::new(&config.general.wallet_private_key, 137)?;
    info!("Wallet address: {:?}", signer.address());

    let clob_client = ClobClient::new(config.general.polymarket_api_url.clone(), signer);
    let position_sizer = PositionSizer::new(config.position_sizing.clone());
    let executor = Arc::new(OrderExecutor::new(
        clob_client,
        position_sizer,
        config.execution.clone(),
    ));

    // Initialize trade logger
    let logger = Arc::new(TradeLogger::new("trades.jsonl".to_string()));

    // Get tracked trader addresses
    let tracked_addresses = config.traders.get_addresses()?;
    info!("Monitoring {} trader accounts", tracked_addresses.len());

    // Initialize polling monitor
    let poll_interval = Duration::from_secs(config.execution.poll_interval_seconds);
    let mut monitor = PollingMonitor::new(
        config.general.polymarket_api_url.clone(),
        tracked_addresses,
        poll_interval,
    );

    info!("Starting monitoring loop...");

    // Run monitoring loop
    monitor
        .monitor_loop(|trade| {
            let executor = executor.clone();
            let logger = logger.clone();
            let trade = trade.clone(); // Clone trade to move into async block

            // Log detected trade
            if let Err(e) = logger.log_detected_trade(&trade) {
                error!("Failed to log detected trade: {}", e);
            }

            info!(
                "Detected trade: {} - Market: {}, Side: {}, Size: {} USDC",
                trade.id, trade.market_id, trade.side, trade.size_usdc
            );

            // Execute trade asynchronously
            tokio::spawn(async move {
                match executor.get_balance().await {
                    Ok(balance) => {
                        match executor.execute_trade(&trade, balance).await {
                            Ok(_) => {
                                info!("Successfully executed copy trade for {}", trade.id);
                            }
                            Err(e) => {
                                error!("Failed to execute trade {}: {}", trade.id, e);
                                if let Err(log_err) = logger.log_failed_trade(&trade, &e.to_string())
                                {
                                    error!("Failed to log error: {}", log_err);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to get balance: {}", e);
                    }
                }
            });

            Ok(())
        })
        .await?;

    Ok(())
}

/// Run backtesting simulation
async fn run_backtest(config: Config) -> Result<()> {
    info!("Initializing backtest mode...");

    // Create backtest engine
    let mut engine = BacktestEngine::new(config.backtest.clone(), config.position_sizing.clone());

    info!("Running backtest simulation...");
    let results = engine.run().await?;

    // Print results
    println!("{}", results.format_report());

    // Save results to file
    let results_json = serde_json::to_string_pretty(&results)?;
    std::fs::write("backtest_results.json", results_json)?;
    info!("Results saved to backtest_results.json");

    Ok(())
}
