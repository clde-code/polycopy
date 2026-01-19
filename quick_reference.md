# Polymarket Copy-Trading Bot - Quick Reference Guide

## What You're Building

A **self-hosted Rust application** that:
1. **Monitors** specific Polymarket trader accounts in real-time
2. **Detects** their trades and executes matching orders at market prices
3. **Sizes positions** intelligently (max absolute size OR max relative to your portfolio)
4. **Simulates** the strategy with backtesting to validate profitability
5. **Accounts for** real-world costs: fees, slippage, partial fills

---

## Architecture at a Glance

```
Trader Activity Feed → Trade Detection → Position Sizing → Order Execution
                            ↓
                      Trade Filtering
                      (size, market, type)
                            
Live Mode:              Backtest Mode:
- RTDS WebSocket       - Load historical data
- API Polling          - Simulate execution
- Real Orders          - Calculate metrics
                       - Export results
```

---

## Configuration Parameters

### Must-Configure:
```toml
[traders]
tracked_accounts = ["0xABC...", "0xDEF..."]  # Which traders to copy

[position_sizing]
max_position_size_absolute = 1000.0  # Max $1000 per trade
max_position_size_relative = 0.1     # Max 10% of my balance per trade
strategy = "hybrid"                  # Use both limits (absolute is hard cap)

[execution]
order_type = "FOK"                   # Fill or Kill (immediate execution)
min_trade_size_usdc = 5.0            # Skip tiny trades
max_trade_size_usdc = 50000.0        # Skip massive trades
```

### For Backtesting:
```toml
[backtest]
start_date = "2025-06-01"
end_date = "2025-12-31"
initial_balance_usdc = 10000.0
slippage_model = "linear"  # Linear impact based on order depth
```

---

## How Position Sizing Works

### Example 1: Absolute Strategy
- Config: `max_position_size_absolute = 1000`
- Trader buys $500 of YES shares
- **You execute**: $500 (below limit)

### Example 2: Relative Strategy
- Config: `max_position_size_relative = 0.1` (10% of portfolio)
- Your balance: $10,000 → 10% = $1,000
- Trader buys $2,000 of YES shares
- **You execute**: $1,000 (limited by relative sizing)

### Example 3: Hybrid Strategy (Recommended)
- Config: `max_position_size_absolute = 1000, max_position_size_relative = 0.1`
- Your balance: $50,000 → 10% = $5,000
- Trader buys $3,000 of YES shares
- **You execute**: $1,000 (absolute limit takes priority)

---

## Two Monitoring Approaches

### Approach A: WebSocket RTDS (Fastest)
- **Latency**: ~100-500ms
- **Reliability**: Depends on WebSocket stability
- **Cost**: Free
- **Code**: Connect to Polymarket's real-time stream, subscribe to `activity` topic

### Approach B: Polling API (Most Reliable)
- **Latency**: ~1-5 seconds per poll
- **Reliability**: Very high
- **Cost**: Free (rate-limited)
- **Code**: Query trader positions every 1-5 seconds, detect deltas

**Recommendation**: Start with polling, upgrade to RTDS after proving concept.

---

## Order Execution Flow

1. **Detect trade** from monitored trader
2. **Calculate position size** based on config + current balance
3. **Sign order** using EIP-712 (wallet ownership proof)
4. **Send to CLOB API** with market price
5. **Poll for confirmation** until filled or timeout
6. **Log trade** to database for later analysis

---

## Backtesting Simulation

### What It Does:
- Loads historical trades from Polymarket
- Replays each trade the target trader made
- Simulates YOUR execution with realistic slippage
- Calculates P&L assuming you held positions until end of backtest

### Key Metrics Calculated:
- **Win Rate**: % of trades that were profitable
- **Total P&L**: Sum of all profits/losses
- **ROI**: Return on initial investment (%)
- **Max Drawdown**: Largest portfolio peak-to-trough decline
- **Sharpe Ratio**: Risk-adjusted returns
- **Profit Factor**: (Avg Win) / (Avg Loss)

### Slippage Models:

**Linear Model** (Recommended):
```
actual_price = quote_price + (order_size / depth_coefficient)
```
- You provide: `depth_coefficient` (typically 50,000 - 200,000)
- Larger orders = more slippage

**Percentage Model**:
```
actual_price = quote_price × (1 + slippage_rate)
```
- You provide: `slippage_rate` (e.g., 0.005 = 0.5% slippage)

---

## Running the Bot

### Live Mode (Real Trading)
```bash
WALLET_PK="0x..." cargo run --release -- --config config.toml
```
- Reads your private key from env var (NEVER hardcode!)
- Connects to Polymarket
- Monitors trader accounts
- Executes real orders

### Backtest Mode (Simulation)
```bash
cargo run --release -- --config config.toml --mode backtest
```
- Loads historical data
- Simulates your copy trades
- Produces performance report
- No real orders placed

---

## Polymarket Technical Details

### Order Types:
- **FOK** (Fill or Kill): Execute immediately or cancel
- **GTC** (Good Till Cancelled): Stay on book until filled or manually cancelled
- **GTD** (Good Till Date): Stay on book until expiration

### Fee Structure:
- **Current**: 0 bps (makers & takers)
- **Future**: Subject to change (monitor docs.polymarket.com)
- **Simulation**: Parameterizable for different scenarios

### Tick Sizes:
- Different markets have different minimum price increments
- 0.1, 0.01, 0.001, 0.0001 (most common)
- SDK handles automatic rounding

### Share Model:
- 1 YES + 1 NO = $1.00 USDC (always)
- Prices range from 0.00 to 1.00
- Probabilities = prices (e.g., 0.25 = 25% chance)

---

## Security Checklist

- [ ] **Private key in env var** (not in config file or code)
- [ ] **Use test net first** (before mainnet capital)
- [ ] **Start with small position sizes** (verify execution works)
- [ ] **Monitor all trades** (logs should show every order)
- [ ] **Backtest your parameters** (before going live)
- [ ] **Set hard caps** (max_position_size_absolute prevents runaway losses)
- [ ] **Enable notifications** (Slack/email on errors)

---

## Project Structure

```
polymarket-copy-trader/
├── src/
│   ├── main.rs                    # Entry point, mode selector
│   ├── config.rs                  # Load & parse config
│   ├── models.rs                  # Data structures
│   ├── monitoring/
│   │   ├── tracker.rs             # Monitor trader activity
│   │   └── detector.rs            # Detect position changes
│   ├── execution/
│   │   ├── signer.rs              # EIP-712 signing
│   │   ├── clob_client.rs         # Polymarket API client
│   │   └── position_sizer.rs      # Calculate order size
│   └── backtest/
│       ├── engine.rs              # Backtesting loop
│       ├── simulator.rs           # Trade execution sim
│       └── metrics.rs             # Calculate P&L
├── Cargo.toml                     # Dependencies
└── config.toml                    # Configuration
```

---

## Key Dependencies

```rust
// Blockchain
ethers = "2.0"              // Web3, signing, contracts

// Async
tokio = "1.0"               // Async runtime
tokio-tungstenite = "0.21"  // WebSocket

// HTTP
reqwest = "0.11"            // REST API client

// Data
serde/serde_json = "1.0"    // JSON serialization
rust_decimal = "1.35"       // Precise decimal math
chrono = "0.4"              // Timestamps
```

---

## Execution Timeline

| Phase | Duration | Focus |
|-------|----------|-------|
| 1. Foundation | Weeks 1-2 | Project setup, config, data models |
| 2. Execution | Weeks 3-4 | EIP-712 signing, CLOB API, order placement |
| 3. Monitoring | Weeks 5-6 | Trader tracking, trade detection, logging |
| 4. Simulation | Weeks 7-8 | Backtesting engine, slippage, metrics |
| 5. Polish | Weeks 9-10 | Error handling, optimization, testing |

**MVP achievable in 4-5 weeks** (phases 1-3 + basic backtest).

---

## What's Different from Existing Bots

| Feature | Existing | Your Bot |
|---------|----------|----------|
| Language | Python/JS | Rust (faster, safer, type-safe) |
| Hosting | Cloud-dependent | Self-hosted (your server) |
| Backtesting | Basic/none | Comprehensive with real slippage |
| Position sizing | Simple ratio | Absolute + relative hybrid |
| Monitoring | Single method | Multiple options (polling + RTDS) |
| Extensibility | Limited | Fully modular architecture |

---

## Troubleshooting

### "Order rejected: INVALID_ORDER_NOT_ENOUGH_BALANCE"
- Check your USDC allowance on Polymarket contract
- Ensure sufficient USDC in wallet

### "Order timed out"
- Market may be illiquid
- Increase `order_confirmation_timeout_ms` in config
- Or reduce position size

### "No trades detected"
- Verify trader addresses are correct (0x prefix)
- Check polling interval (1 second default)
- Ensure network connectivity

### "Slippage too high in backtest"
- Increase `depth_coefficient` (less impact)
- Or validate historical order book depth data

---

## Next Steps

1. **Clone repo** (or create from scratch with provided outline)
2. **Set up Rust** environment (rustc 1.70+)
3. **Create `config.toml`** with your parameters
4. **Run backtest** with historical data to validate strategy
5. **Start live** with tiny position sizes (e.g., $10 trades)
6. **Monitor & iterate** based on performance

---

## Resources

- **Polymarket Docs**: https://docs.polymarket.com
- **CLOB API**: https://docs.polymarket.com/developers/CLOB/introduction
- **Ethers-rs**: https://docs.rs/ethers/
- **Rust Book**: https://doc.rust-lang.org/book/

