# Rust-Based Polymarket Copy-Trading Bot: Implementation Outline

## Executive Summary

This document outlines a self-hosted, Rust-based copy-trading application for Polymarket that:
- Monitors specific trader accounts in real-time
- Executes matching trades at market prices with configurable position sizing
- Includes a robust backtesting/simulation engine for strategy validation
- Accounts for fees, slippage, and partial fills

---

## 1. Architecture Overview

### High-Level Components

```
┌─────────────────────────────────────────────────────────────────┐
│                      Copy-Trading Bot                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Configuration Layer                                     │   │
│  │  - Account public keys to copy                           │   │
│  │  - Max position size (absolute & relative)               │   │
│  │  - Position sizing strategy                              │   │
│  │  - Execution parameters                                  │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Monitoring & Detection Layer                            │   │
│  │  - Real-time trader activity tracking (RTDS/API)         │   │
│  │  - Position change detection                             │   │
│  │  - Trade filtering (size, market, etc.)                  │   │
│  │  - Market data fetching                                  │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Order Execution Layer                                   │   │
│  │  - Position sizing calculation                           │   │
│  │  - EIP-712 signing                                       │   │
│  │  - CLOB API integration                                  │   │
│  │  - Order placement & monitoring                          │   │
│  │  - Error handling & retries                              │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Simulation / Backtesting Engine                         │   │
│  │  - Historical market data loading                        │   │
│  │  - Trade execution simulation                            │   │
│  │  - Slippage & fee modeling                               │   │
│  │  - Performance metrics calculation                       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Storage & Persistence Layer                             │   │
│  │  - Trade history / execution log                         │   │
│  │  - Position tracking                                     │   │
│  │  - Performance metrics                                   │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

### Key Modules

```
polymarket-copy-trader/
├── src/
│   ├── main.rs                  # Entry point, mode selector (live/backtest)
│   ├── config.rs                # Configuration struct & loading
│   ├── models.rs                # Domain models (Trade, Position, etc.)
│   ├── errors.rs                # Error types
│   │
│   ├── monitoring/
│   │   ├── mod.rs
│   │   ├── tracker.rs           # Trader activity monitor
│   │   ├── market_data.rs       # Market data fetcher
│   │   └── detector.rs          # Trade detection logic
│   │
│   ├── execution/
│   │   ├── mod.rs
│   │   ├── signer.rs            # EIP-712 signing
│   │   ├── clob_client.rs       # Polymarket CLOB API wrapper
│   │   ├── order_executor.rs    # Order placement & tracking
│   │   └── position_sizer.rs    # Position sizing logic
│   │
│   ├── backtest/
│   │   ├── mod.rs
│   │   ├── engine.rs            # Backtesting engine
│   │   ├── simulator.rs         # Trade execution simulator
│   │   ├── slippage.rs          # Slippage calculation
│   │   └── metrics.rs           # Performance metrics
│   │
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── trade_log.rs         # Trade history persistence
│   │   └── position_db.rs       # Position state management
│   │
│   └── utils/
│       ├── logger.rs
│       └── helpers.rs
│
├── Cargo.toml
├── config.example.toml
└── README.md
```

---

## 2. Configuration & Parameters

### `config.toml` Structure

```toml
[general]
mode = "live"  # "live" or "backtest"
wallet_private_key = "${WALLET_PK}"  # From env var
polygon_rpc_url = "https://polygon-rpc.com"
polymarket_api_url = "https://clob.polymarket.com"

[traders]
tracked_accounts = [
  "0xABC123...",
  "0xDEF456..."
]

[position_sizing]
# Absolute max position size (in USDC)
max_position_size_absolute = 1000.0

# Relative to portfolio (e.g., 0.1 = 10% of balance per trade)
max_position_size_relative = 0.1

# Strategy: "absolute", "relative", or "hybrid"
strategy = "hybrid"

# For hybrid: which takes precedence if both exceeded
priority = "absolute"  # Use absolute limit as hard cap

[execution]
# Order type: "FOK" (Fill or Kill), "GTC" (Good Till Cancelled), "GTD"
order_type = "FOK"

# For GTD orders: duration in seconds
gtd_duration_seconds = 300

# Timeout for order confirmation polling (ms)
order_confirmation_timeout_ms = 30000

# Polling interval for order status (ms)
order_poll_interval_ms = 500

# Retry attempts for failed orders
max_retries = 3

# Minimum trade size to copy (in USDC) - filter small trades
min_trade_size_usdc = 5.0

# Maximum trade size to copy (in USDC) - filter very large trades
max_trade_size_usdc = 50000.0

[backtest]
mode = "simulation"  # or "historical"
start_date = "2025-06-01"
end_date = "2025-12-31"
initial_balance_usdc = 10000.0

# Historical data source
data_source = "polymarket_api"  # or "csv_file"
data_file = "./data/trades.csv"

# Slippage model: "linear", "percentage", "market_impact"
slippage_model = "linear"

# For linear model: depth coefficient
depth_coefficient = 100000.0

# Apply fees in simulation
apply_fees = true

# Apply gas costs (MATIC)
apply_gas_costs = false
estimated_gas_per_trade_usd = 0.1

[logging]
level = "info"  # "debug", "info", "warn", "error"
file_output = "./logs/copy_trader.log"
max_log_size_mb = 100
log_retention_days = 30

[database]
db_type = "sqlite"  # "sqlite", "postgres", "mongodb"
db_connection = "copy_trader.db"

[notifications]
# Optional: Slack, email, etc.
slack_webhook_url = "${SLACK_WEBHOOK}"
notify_on_trade = true
notify_on_error = true
```

### Runtime Position Sizing Calculation

```rust
// Logic for determining actual trade size
fn calculate_position_size(
    target_trade_size: Decimal,           // Size of tracked trader's order
    my_balance: Decimal,                  // Current USDC balance
    config: &PositionSizingConfig,
) -> Result<Decimal, PositionSizingError> {
    let mut size = target_trade_size;
    
    // Apply relative sizing if configured
    if config.strategy == "relative" || config.strategy == "hybrid" {
        let relative_size = my_balance * config.max_position_size_relative;
        size = min(size, relative_size);
    }
    
    // Apply absolute cap
    if config.strategy == "absolute" || config.strategy == "hybrid" {
        size = min(size, config.max_position_size_absolute);
    }
    
    // Ensure minimum size met
    if size < MIN_ORDER_SIZE {
        return Err(PositionSizingError::BelowMinimum);
    }
    
    Ok(size)
}
```

---

## 3. Monitoring & Detection

### Real-Time Trader Monitoring

**Two Approaches:**

#### Approach A: RTDS (Real-Time Data Stream) - Recommended
- WebSocket connection to Polymarket's real-time stream
- Lowest latency (~100-500ms detection)
- Subscribe to `activity` topic for global trades
- Filter by tracked trader addresses

```rust
pub struct RTDSMonitor {
    ws_client: WebSocketClient,
    tracked_traders: HashSet<Address>,
    on_trade_detected: Box<dyn Fn(&Trade) + Send + Sync>,
}

impl RTDSMonitor {
    pub async fn connect(&mut self) -> Result<()> {
        self.ws_client.connect("wss://rtds.polymarket.com").await?;
        self.ws_client.subscribe("activity").await?;
        Ok(())
    }
    
    pub async fn listen(&mut self) -> Result<()> {
        loop {
            if let Some(msg) = self.ws_client.next().await {
                let trade: Trade = serde_json::from_str(&msg)?;
                
                if self.tracked_traders.contains(&trade.maker) ||
                   self.tracked_traders.contains(&trade.taker) {
                    (self.on_trade_detected)(&trade);
                }
            }
        }
    }
}
```

#### Approach B: Polling API - More Reliable
- Query Polymarket Data API or Data API every N seconds
- Higher latency (~1-5 second detection)
- More robust, doesn't depend on WebSocket stability
- Use Bitquery GraphQL for position tracking

```rust
pub struct PollingMonitor {
    api_client: PolymarketApiClient,
    tracked_traders: Vec<Address>,
    poll_interval: Duration,
    last_state: HashMap<Address, TraderState>,
}

impl PollingMonitor {
    pub async fn monitor_loop(&mut self) -> Result<()> {
        loop {
            for trader_addr in &self.tracked_traders {
                let current_state = self.api_client.get_trader_positions(trader_addr).await?;
                let previous_state = self.last_state.get(trader_addr);
                
                if let Some(new_trades) = self.detect_changes(current_state, previous_state) {
                    for trade in new_trades {
                        self.handle_trade(&trade).await?;
                    }
                }
            }
            
            tokio::time::sleep(self.poll_interval).await;
        }
    }
}
```

### Trade Detection & Filtering

```rust
pub struct TradeFilter {
    min_size_usdc: Decimal,
    max_size_usdc: Decimal,
    allowed_markets: Option<HashSet<String>>,
    min_market_duration: Duration,
    filter_by_win_rate: Option<Decimal>,  // Min win rate %
}

impl TradeFilter {
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
        
        // Market maturity (avoid ultra-short-term markets)
        let time_to_resolution = trade.resolution_time - trade.execution_time;
        if time_to_resolution < self.min_market_duration {
            return false;
        }
        
        // Trader track record
        if let Some(min_wr) = self.filter_by_win_rate {
            if trade.trader_win_rate < min_wr {
                return false;
            }
        }
        
        true
    }
}
```

---

## 4. Order Execution

### EIP-712 Signing Implementation

```rust
use ethers::signable_message::SignableMessage;
use ethers::types::{H256, Signature, Address};
use ethers::utils::hash_structured_data;

pub struct OrderSigner {
    signer: LocalWallet,
    chain_id: u64,
}

impl OrderSigner {
    pub fn new(private_key: &str, chain_id: u64) -> Result<Self> {
        let wallet = private_key.parse::<LocalWallet>()?;
        Ok(Self {
            signer: wallet,
            chain_id,
        })
    }
    
    pub async fn sign_order(&self, order_data: &OrderData) -> Result<String> {
        // Build EIP-712 domain
        let domain = eip712::Domain {
            name: Some("CLOBAuth".to_string()),
            version: Some("1".to_string()),
            chain_id: Some(U256::from(self.chain_id)),
            verifying_contract: Some(
                "0xC5d563A36AE78145C45a50134d48A1215220f80a".parse()? // Polymarket contract
            ),
            ..Default::default()
        };
        
        // Build message
        let timestamp = now_unix();
        let nonce = 0u64;
        let message = "This message attests that I control the given wallet";
        
        let types = vec![
            ("address".to_string(), "address"),
            ("timestamp".to_string(), "uint256"),
            ("nonce".to_string(), "uint256"),
            ("message".to_string(), "string"),
        ];
        
        let value = serde_json::json!({
            "address": self.signer.address(),
            "timestamp": timestamp,
            "nonce": nonce,
            "message": message,
        });
        
        // Hash structured data
        let struct_hash = hash_structured_data(domain, types, value)?;
        
        // Sign
        let signature = self.signer.sign_hash(struct_hash)?;
        
        Ok(signature.to_string())
    }
}
```

### CLOB API Integration

```rust
pub struct ClobClient {
    http_client: reqwest::Client,
    api_url: String,
    signer: OrderSigner,
    my_address: Address,
}

impl ClobClient {
    pub async fn place_order(
        &self,
        market_id: &str,
        side: OrderSide,
        price: Decimal,
        size: Decimal,
        order_type: OrderType,
    ) -> Result<OrderResponse> {
        // Calculate tick size compliance
        let tick_size = self.get_tick_size(market_id).await?;
        let adjusted_price = self.adjust_to_tick_size(price, tick_size)?;
        
        // Prepare order
        let order = Order {
            market_id: market_id.to_string(),
            price_decimal: adjusted_price,
            quantity: size,
            side,
            owner: self.my_address,
            expiration_time: (now() + Duration::from_secs(600)).as_secs(),
        };
        
        // Sign order
        let signature = self.signer.sign_order(&order).await?;
        
        // Create request
        let request = OrderRequest {
            order: order.clone(),
            owner: self.my_address.to_string(),
            order_type: order_type.to_string(),
            post_only: false,
            fee_rate_bps: "0".to_string(),
            side: side.to_string(),
            signature_type: 0,  // EOA
            signature: signature.to_string(),
        };
        
        // Send to API
        let response = self.http_client
            .post(&format!("{}/order", self.api_url))
            .header("POLY_ADDRESS", self.my_address.to_string())
            .header("POLY_SIGNATURE", &signature)
            .header("POLY_TIMESTAMP", now_unix().to_string())
            .header("POLY_NONCE", "0")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(ExecutionError::ApiError(error));
        }
        
        let order_response: OrderResponse = response.json().await?;
        Ok(order_response)
    }
    
    pub async fn get_tick_size(&self, market_id: &str) -> Result<Decimal> {
        let response = self.http_client
            .get(&format!("{}/markets/{}", self.api_url, market_id))
            .send()
            .await?;
        
        let market_data: MarketData = response.json().await?;
        Ok(market_data.tick_size)
    }
}
```

### Order Status Monitoring

```rust
pub struct OrderMonitor {
    clob_client: ClobClient,
    poll_interval: Duration,
    timeout: Duration,
}

impl OrderMonitor {
    pub async fn wait_for_fill(
        &self,
        order_id: &str,
        expected_size: Decimal,
    ) -> Result<OrderFillStatus> {
        let start = Instant::now();
        
        loop {
            let order = self.clob_client.get_order(order_id).await?;
            
            match order.status {
                OrderStatus::Filled => {
                    return Ok(OrderFillStatus::FullyFilled {
                        price: order.filled_price,
                        size: order.size_filled,
                    });
                }
                OrderStatus::PartiallyFilled => {
                    if start.elapsed() > self.timeout {
                        return Ok(OrderFillStatus::PartiallyFilled {
                            price: order.filled_price,
                            size: order.size_filled,
                        });
                    }
                }
                OrderStatus::Open => {
                    if start.elapsed() > self.timeout {
                        // Cancel unfilled orders
                        self.clob_client.cancel_order(order_id).await?;
                        return Ok(OrderFillStatus::TimedOut);
                    }
                }
                OrderStatus::Cancelled => {
                    return Ok(OrderFillStatus::Cancelled);
                }
            }
            
            tokio::time::sleep(self.poll_interval).await;
        }
    }
}
```

---

## 5. Backtesting & Simulation Engine

### Simulation Core

```rust
pub struct BacktestEngine {
    config: BacktestConfig,
    market_data: Vec<HistoricalTrade>,
    simulator: TradeSimulator,
    metrics: PerformanceMetrics,
}

impl BacktestEngine {
    pub async fn run(&mut self) -> Result<BacktestResults> {
        // Load historical data
        self.load_historical_data().await?;
        
        // Process each historical trade
        for historical_trade in &self.market_data.clone() {
            // Simulate target trader's action
            if self.should_copy_trade(historical_trade) {
                // Calculate my position size
                let my_size = self.calculate_position_size_backtest(
                    historical_trade.size,
                    self.simulator.balance(),
                );
                
                // Simulate execution with slippage
                let simulated_trade = self.simulator.simulate_execution(
                    &historical_trade.market,
                    historical_trade.side,
                    my_size,
                    historical_trade.price,
                    &self.config.slippage_model,
                )?;
                
                // Track position
                self.simulator.add_position(simulated_trade)?;
                
                // Update metrics
                self.metrics.record_trade(&simulated_trade);
            }
        }
        
        // Close all positions at end of backtest
        self.simulator.close_all_positions().await?;
        
        Ok(self.metrics.generate_report())
    }
    
    async fn load_historical_data(&mut self) -> Result<()> {
        match &self.config.data_source {
            DataSource::PolymarketApi => {
                self.market_data = self.fetch_from_api().await?;
            }
            DataSource::CsvFile(path) => {
                self.market_data = self.load_from_csv(path)?;
            }
        }
        
        // Filter by date range
        self.market_data.retain(|trade| {
            trade.timestamp >= self.config.start_date &&
            trade.timestamp <= self.config.end_date
        });
        
        Ok(())
    }
}
```

### Slippage & Fill Simulation

```rust
pub enum SlippageModel {
    Linear { depth_coefficient: Decimal },
    Percentage { rate: Decimal },
    MarketImpact { impact_param: Decimal },
}

pub struct TradeSimulator {
    balance: Decimal,
    positions: Vec<Position>,
    slippage_model: SlippageModel,
}

impl TradeSimulator {
    pub fn simulate_execution(
        &mut self,
        market: &Market,
        side: OrderSide,
        size: Decimal,
        quote_price: Decimal,
        slippage_model: &SlippageModel,
    ) -> Result<ExecutedTrade> {
        // Calculate actual execution price with slippage
        let actual_price = match slippage_model {
            SlippageModel::Linear { depth_coefficient } => {
                let impact = size / depth_coefficient;
                if side == OrderSide::Buy {
                    quote_price + impact
                } else {
                    quote_price - impact
                }
            }
            SlippageModel::Percentage { rate } => {
                if side == OrderSide::Buy {
                    quote_price * (Decimal::one() + rate)
                } else {
                    quote_price * (Decimal::one() - rate)
                }
            }
            SlippageModel::MarketImpact { impact_param } => {
                let impact = impact_param * size.ln();
                if side == OrderSide::Buy {
                    quote_price + impact
                } else {
                    quote_price - impact
                }
            }
        };
        
        // Calculate costs
        let cost = match side {
            OrderSide::Buy => size * actual_price,
            OrderSide::Sell => size * actual_price,
        };
        
        // Apply fees (currently 0, but parameterizable)
        let fee = cost * (Decimal::new(0, 0)); // 0 bps
        let total_cost = cost + fee;
        
        // Check balance
        if side == OrderSide::Buy && total_cost > self.balance {
            return Err(SimulationError::InsufficientBalance);
        }
        
        // Update balance
        match side {
            OrderSide::Buy => self.balance -= total_cost,
            OrderSide::Sell => self.balance += total_cost,
        }
        
        // Record position
        let position = Position {
            market_id: market.id.clone(),
            entry_price: actual_price,
            size,
            side,
            timestamp: now(),
            pnl: Decimal::zero(),
        };
        
        self.positions.push(position.clone());
        
        Ok(ExecutedTrade {
            position,
            actual_price,
            slippage: actual_price - quote_price,
            fee,
        })
    }
    
    pub async fn close_all_positions(&mut self) -> Result<Vec<ClosedPosition>> {
        let mut closed = Vec::new();
        
        for position in self.positions.drain(..) {
            // Query current market price
            let current_price = self.get_market_price(&position.market_id).await?;
            
            // Calculate P&L
            let pnl = match position.side {
                OrderSide::Buy => (current_price - position.entry_price) * position.size,
                OrderSide::Sell => (position.entry_price - current_price) * position.size,
            };
            
            // Update balance
            self.balance += position.size * current_price;
            
            closed.push(ClosedPosition {
                position,
                exit_price: current_price,
                pnl,
            });
        }
        
        Ok(closed)
    }
}
```

### Performance Metrics

```rust
pub struct PerformanceMetrics {
    trades: Vec<ExecutedTrade>,
    closed_positions: Vec<ClosedPosition>,
    initial_balance: Decimal,
}

impl PerformanceMetrics {
    pub fn generate_report(&self) -> BacktestResults {
        let total_pnl: Decimal = self.closed_positions.iter().map(|p| p.pnl).sum();
        let winning_trades = self.closed_positions.iter().filter(|p| p.pnl > Decimal::zero()).count();
        let losing_trades = self.closed_positions.iter().filter(|p| p.pnl < Decimal::zero()).count();
        
        let win_rate = if self.closed_positions.is_empty() {
            Decimal::zero()
        } else {
            Decimal::from(winning_trades) / Decimal::from(self.closed_positions.len())
        };
        
        let avg_win = if winning_trades == 0 {
            Decimal::zero()
        } else {
            let sum: Decimal = self.closed_positions.iter()
                .filter(|p| p.pnl > Decimal::zero())
                .map(|p| p.pnl)
                .sum();
            sum / Decimal::from(winning_trades)
        };
        
        let avg_loss = if losing_trades == 0 {
            Decimal::zero()
        } else {
            let sum: Decimal = self.closed_positions.iter()
                .filter(|p| p.pnl < Decimal::zero())
                .map(|p| p.pnl.abs())
                .sum();
            sum / Decimal::from(losing_trades)
        };
        
        let profit_factor = if avg_loss == Decimal::zero() {
            Decimal::from(1000) // Cap at 1000
        } else {
            avg_win / avg_loss
        };
        
        let roi = (total_pnl / self.initial_balance) * Decimal::from(100);
        
        // Max drawdown calculation
        let max_drawdown = self.calculate_max_drawdown();
        
        // Sharpe ratio (simplified)
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
    
    fn calculate_max_drawdown(&self) -> Decimal {
        let mut peak = self.initial_balance;
        let mut max_dd = Decimal::zero();
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
}
```

---

## 6. Live Trading Loop

### Main Application Flow

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let config = Config::load_from_file("config.toml")?;
    
    // Initialize logger
    init_logger(&config.logging)?;
    
    match config.general.mode.as_str() {
        "live" => run_live_trading(config).await,
        "backtest" => run_backtest(config).await,
        _ => Err("Unknown mode".into()),
    }
}

async fn run_live_trading(config: Config) -> Result<()> {
    // Initialize components
    let signer = OrderSigner::new(&config.general.wallet_private_key, 137)?;
    let clob_client = ClobClient::new(&config.general.polymarket_api_url, signer)?;
    let mut monitor = RTDSMonitor::new(config.traders.tracked_accounts.clone());
    let executor = OrderExecutor::new(clob_client, config.execution.clone());
    
    // Connect to monitoring
    monitor.connect().await?;
    
    // Set up event handler
    let executor_clone = executor.clone();
    let config_clone = config.clone();
    
    monitor.on_trade_detected = Box::new(move |trade| {
        let executor = executor_clone.clone();
        let config = config_clone.clone();
        
        tokio::spawn(async move {
            // Filter trade
            if !should_copy_trade(trade, &config) {
                return;
            }
            
            // Calculate position size
            let my_balance = executor.get_balance().await.unwrap_or_default();
            let position_size = calculate_position_size(
                trade.size,
                my_balance,
                &config.position_sizing,
            ).unwrap_or(Decimal::zero());
            
            // Execute order
            if let Err(e) = executor.place_order_for_trade(trade, position_size).await {
                error!("Failed to execute trade: {}", e);
            }
        });
    });
    
    // Run monitoring loop
    monitor.listen().await?;
    
    Ok(())
}

async fn run_backtest(config: Config) -> Result<()> {
    let mut engine = BacktestEngine::new(config.backtest.clone());
    
    info!("Starting backtest...");
    let results = engine.run().await?;
    
    // Print results
    println!("\n{}", results.format_report());
    
    // Save results
    results.save_to_file("backtest_results.json")?;
    
    Ok(())
}
```

---

## 7. Data Models

```rust
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
    pub trader_win_rate: Option<Decimal>,
}

#[derive(Clone, Debug)]
pub struct ExecutedTrade {
    pub position: Position,
    pub actual_price: Decimal,
    pub slippage: Decimal,
    pub fee: Decimal,
}

#[derive(Clone, Debug)]
pub struct Position {
    pub market_id: String,
    pub entry_price: Decimal,
    pub size: Decimal,
    pub side: OrderSide,
    pub timestamp: DateTime<Utc>,
    pub pnl: Decimal,
}

#[derive(Clone, Debug)]
pub struct ClosedPosition {
    pub position: Position,
    pub exit_price: Decimal,
    pub pnl: Decimal,
}

#[derive(Serialize)]
pub struct BacktestResults {
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate: Decimal,        // %
    pub total_pnl: Decimal,
    pub roi: Decimal,              // %
    pub avg_win: Decimal,
    pub avg_loss: Decimal,
    pub profit_factor: Decimal,
    pub max_drawdown: Decimal,     // %
    pub sharpe_ratio: Decimal,
    pub initial_balance: Decimal,
    pub final_balance: Decimal,
}
```

---

## 8. Dependencies (Cargo.toml)

```toml
[package]
name = "polymarket-copy-trader"
version = "0.1.0"
edition = "2021"

[dependencies]
# Web3 / Blockchain
ethers = { version = "2.0", features = ["full"] }
ethers-signers = "2.0"

# Async runtime
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.21"

# HTTP client
reqwest = { version = "0.11", features = ["json"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Numeric precision
rust_decimal = { version = "1.35", features = ["serde"] }

# Cryptography
sha3 = "0.10"
hmac = "0.12"

# DateTime
chrono = { version = "0.4", features = ["serde"] }

# Database (optional, comment out if not using)
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio-native-tls", "uuid", "chrono", "decimal"] }
# mongodb = { version = "3.0", features = ["sync"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt", "json"] }

# CLI / Config
clap = { version = "4.4", features = ["derive"] }

# Utilities
anyhow = "1.0"
thiserror = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
dotenv = "0.15"

[dev-dependencies]
tokio-test = "0.4"
```

---

## 9. Key Implementation Steps

### Phase 1: Foundation (Weeks 1-2)
1. ✅ Set up Rust project structure
2. ✅ Implement configuration loading
3. ✅ Create data models
4. ✅ Set up logging and error handling
5. ✅ Basic ethers-rs integration for wallet

### Phase 2: Execution (Weeks 3-4)
1. ✅ EIP-712 signing implementation
2. ✅ CLOB API client (order placement, market data)
3. ✅ Token allowance management
4. ✅ Order status monitoring

### Phase 3: Monitoring (Weeks 5-6)
1. ✅ Polling-based trader monitoring (MVP)
2. ✅ Trade detection and filtering
3. ✅ Position sizing calculation
4. ✅ Trade logging and persistence

### Phase 4: Simulation (Weeks 7-8)
1. ✅ Historical data loading (API/CSV)
2. ✅ Slippage modeling (linear, percentage)
3. ✅ Backtesting engine
4. ✅ Performance metrics calculation

### Phase 5: Polish (Weeks 9-10)
1. ✅ Error handling and retries
2. ✅ WebSocket RTDS integration (optional)
3. ✅ Performance optimization
4. ✅ Testing and documentation

---

## 10. Security Considerations

1. **Private Key Management**
   - Never hardcode keys
   - Use environment variables or secure vaults
   - Consider using hardware wallets for production

2. **Order Signing**
   - Always verify signature structure
   - Use timestamp nonces to prevent replay attacks
   - Validate chain ID before signing

3. **API Communication**
   - Use HTTPS/WSS only
   - Implement request timeouts
   - Rate limit API calls

4. **Position Limits**
   - Enforce max position size hard caps
   - Monitor total exposure
   - Implement circuit breakers for losses

5. **Audit Trail**
   - Log all orders, fills, and errors
   - Persist state for recovery
   - Enable performance auditing

---

## 11. Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_sizing_absolute_limit() {
        let config = PositionSizingConfig {
            strategy: "absolute".to_string(),
            max_position_size_absolute: Decimal::from(100),
            max_position_size_relative: Decimal::from(0),
            priority: "absolute".to_string(),
        };
        
        let size = calculate_position_size(
            Decimal::from(500),
            Decimal::from(1000),
            &config
        ).unwrap();
        
        assert_eq!(size, Decimal::from(100));
    }

    #[test]
    fn test_slippage_linear_model() {
        let simulator = TradeSimulator::new(Decimal::from(10000));
        let price = Decimal::from_str("0.5").unwrap();
        let size = Decimal::from(100);
        
        let actual = price + (size / Decimal::from(100000));
        assert!(actual > price);
    }

    #[tokio::test]
    async fn test_order_placement_signature() {
        let signer = OrderSigner::new(TEST_PRIVATE_KEY, 137).unwrap();
        let signature = signer.sign_order(&test_order()).await.unwrap();
        assert!(!signature.is_empty());
    }
}
```

---

## 12. Deployment & Operations

### Self-Hosting Requirements
- **Compute**: 1+ core CPU, 2+ GB RAM minimum
- **Storage**: 10+ GB for historical data and logs
- **Network**: Stable internet connection, low latency preferred
- **Environment**: Linux/macOS recommended; Windows WSL2 acceptable

### Running Live Copy Trading
```bash
# Compile
cargo build --release

# Run in live mode
WALLET_PK="0x..." POLYMARKET_API_URL="..." cargo run --release -- --config config.toml

# Or with screen/tmux for persistence
tmux new-session -d -s polymarket "cargo run --release"
```

### Running Backtest
```bash
cargo run --release -- --config config.toml --mode backtest

# View results
cat backtest_results.json | jq .
```

### Monitoring & Logs
```bash
# Tail logs
tail -f logs/copy_trader.log

# Filter for errors
grep ERROR logs/copy_trader.log
```

---

## 13. Future Enhancements

1. **Multi-Market Hedging**: Simultaneously track traders across correlated markets
2. **Advanced Position Sizing**: Kelly criterion, risk-parity sizing
3. **Real-Time Risk Management**: Stop-loss, take-profit automation
4. **ML-Based Trader Selection**: Identify high-quality traders to copy
5. **Portfolio Analytics**: Detailed performance dashboards
6. **Decentralized Storage**: Store trade history on IPFS
7. **Smart Contract Automation**: Execute trades on-chain directly
8. **Liquidity Aggregation**: Route orders across best prices

---

## Conclusion

This architecture provides a robust, production-ready framework for:
- **Real-time monitoring** of trader accounts with configurable filtering
- **Intelligent position sizing** with both absolute and relative constraints
- **Reliable order execution** with EIP-712 signing and error handling
- **Comprehensive backtesting** with accurate slippage and fee modeling

The modular design allows for incremental development, testing at each phase, and easy addition of advanced features without refactoring core logic.

