# Polymarket Copy Trader

A self-hosted, production-ready Rust-based copy-trading bot for Polymarket. Monitor specific trader accounts in real-time and automatically execute matching trades with intelligent position sizing and comprehensive backtesting.

## Features

- **Real-time Monitoring**: Track multiple trader accounts via API polling
- **Intelligent Position Sizing**: Absolute, relative, or hybrid position limits
- **EIP-712 Signing**: Secure order authentication for Polymarket CLOB
- **Comprehensive Backtesting**: Validate strategies with realistic slippage and fee modeling
- **Trade Logging**: Complete audit trail of all detected and executed trades
- **Configurable Filters**: Size limits, market filters, and trader quality metrics

## Quick Start

### Prerequisites

- Rust 1.70+ ([Install Rust](https://rustup.rs/))
- A Polygon wallet with USDC
- Polymarket account

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/polycopy.git
cd polycopy

# Copy example configuration
cp config.example.toml config.toml

# Edit configuration with your settings
nano config.toml
```

### Configuration

Edit `config.toml` to set:

1. **Tracked Traders**: Add wallet addresses to monitor
```toml
[traders]
tracked_accounts = [
    "0x1234567890123456789012345678901234567890"
]
```

2. **Position Sizing**: Set your risk limits
```toml
[position_sizing]
max_position_size_absolute = 1000.0  # Max $1000 per trade
max_position_size_relative = 0.1     # Max 10% of balance
strategy = "hybrid"                  # Use both limits
```

3. **Wallet Private Key** (via environment variable):
```bash
export WALLET_PK="your_private_key_here"
```

### Running Backtest Mode

Test your strategy before going live:

```bash
cargo run --release -- --mode backtest
```

Output:
```
╔══════════════════════════════════════════════════════════════╗
║              BACKTEST RESULTS                                ║
╠══════════════════════════════════════════════════════════════╣
║ Total Trades:                                           50 ║
║ Winning Trades:                                         32 ║
║ Losing Trades:                                          18 ║
║ Win Rate:                                            64.00% ║
╠══════════════════════════════════════════════════════════════╣
║ Initial Balance:                                 10000.00 USDC ║
║ Final Balance:                                   11250.00 USDC ║
║ Total P&L:                                        1250.00 USDC ║
║ ROI:                                                 12.50% ║
╚══════════════════════════════════════════════════════════════╝
```

### Running Live Trading

After validating your strategy:

```bash
export WALLET_PK="your_private_key"
cargo run --release -- --mode live
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   Polymarket Copy Trader                     │
├─────────────────────────────────────────────────────────────┤
│  Configuration → Monitoring → Detection → Sizing → Execution │
│                      ↓                          ↓             │
│                  Logging                   Backtesting       │
└─────────────────────────────────────────────────────────────┘
```

### Core Modules

- **`config`**: Configuration loading and validation
- **`execution`**: EIP-712 signing, CLOB API integration, order execution
- **`monitoring`**: Trader activity tracking and trade detection
- **`backtest`**: Simulation engine with slippage modeling and metrics
- **`storage`**: Trade logging and persistence

## Position Sizing Strategies

### Absolute Strategy
Caps each trade at a fixed USDC amount.

**Example**: Max $1000 per trade
- Trader buys $500 → You execute $500 ✓
- Trader buys $2000 → You execute $1000 (capped)

### Relative Strategy
Caps each trade as a percentage of your balance.

**Example**: Max 10% of portfolio
- Your balance: $10,000
- Trader buys $2000 → You execute $1000 (10% of $10k)
- Your balance: $50,000
- Trader buys $2000 → You execute $2000 (within 10% of $50k)

### Hybrid Strategy (Recommended)
Uses both limits with one taking precedence.

**Example**: Max $1000 absolute OR 10% relative
- Your balance: $50,000 (10% = $5000)
- Trader buys $3000
- **You execute**: $1000 (absolute limit acts as hard cap)

## Configuration Reference

See `config.example.toml` for all available options.

### Key Settings

| Parameter | Description | Example |
|-----------|-------------|---------|
| `mode` | Operating mode | `"live"` or `"backtest"` |
| `tracked_accounts` | Trader addresses to copy | `["0xABC..."]` |
| `max_position_size_absolute` | Hard cap per trade (USDC) | `1000.0` |
| `max_position_size_relative` | Max % of balance per trade | `0.1` (10%) |
| `order_type` | Order execution type | `"FOK"`, `"GTC"`, `"GTD"` |
| `min_trade_size_usdc` | Skip trades smaller than | `5.0` |
| `max_trade_size_usdc` | Skip trades larger than | `50000.0` |
| `slippage_model` | Backtest slippage model | `"linear"`, `"percentage"` |

## Backtesting

The backtest engine simulates your copy-trading strategy on historical data:

1. **Load Historical Data**: From Polymarket API or CSV
2. **Simulate Execution**: Apply position sizing, slippage, and fees
3. **Calculate Metrics**: Win rate, P&L, Sharpe ratio, max drawdown

### Slippage Models

**Linear Model** (Default):
```
actual_price = quote_price + (order_size / depth_coefficient)
```

**Percentage Model**:
```
actual_price = quote_price × (1 + slippage_rate)
```

### Performance Metrics

- **Win Rate**: Percentage of profitable trades
- **ROI**: Return on initial investment
- **Profit Factor**: Average win / Average loss
- **Max Drawdown**: Largest peak-to-trough decline
- **Sharpe Ratio**: Risk-adjusted returns

## Security Best Practices

⚠️ **Critical Security Notes**:

1. **Never commit private keys** - Always use environment variables
2. **Start with small amounts** - Test with $10 trades first
3. **Use testnet first** (if available)
4. **Monitor all trades** - Check `trades.jsonl` regularly
5. **Set hard caps** - Use `max_position_size_absolute` as insurance

### Environment Variables

```bash
export WALLET_PK="your_private_key_here"
export SLACK_WEBHOOK="https://hooks.slack.com/..." # Optional
```

## Logging

All trades are logged to `trades.jsonl`:

```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "trade": {
    "id": "abc123",
    "market_id": "0x...",
    "side": "BUY",
    "price": "0.55",
    "size_usdc": "100"
  },
  "executed": {
    "actual_price": "0.551",
    "slippage": "0.001",
    "fee": "0"
  },
  "success": true
}
```

## Monitoring Approaches

### API Polling (Implemented)
- **Latency**: 1-5 seconds
- **Reliability**: High
- **Implementation**: Queries positions every N seconds

### WebSocket RTDS (Future Enhancement)
- **Latency**: 100-500ms
- **Reliability**: Medium (connection stability)
- **Implementation**: Real-time trade stream

## Troubleshooting

### "Insufficient balance" errors
- Check USDC balance in your Polygon wallet
- Verify USDC allowance for Polymarket contract

### "Order timeout"
- Market may be illiquid
- Increase `order_confirmation_timeout_ms`
- Reduce position size

### "No trades detected"
- Verify trader addresses (include `0x` prefix)
- Check `poll_interval_seconds` (default: 2s)
- Ensure network connectivity

### High slippage in backtest
- Increase `depth_coefficient` (reduces simulated impact)
- Or validate against actual order book depth

## Development

### Running Tests

```bash
cargo test
```

### Building Release Binary

```bash
cargo build --release
./target/release/polymarket-copy-trader --help
```

### Project Structure

```
src/
├── main.rs              # Entry point, mode selector
├── config.rs            # Configuration loading
├── models.rs            # Data structures
├── errors.rs            # Error types
├── execution/           # Order execution
│   ├── signer.rs        # EIP-712 signing
│   ├── clob_client.rs   # Polymarket API
│   ├── position_sizer.rs
│   └── order_executor.rs
├── monitoring/          # Trader monitoring
│   ├── tracker.rs       # Activity tracking
│   └── detector.rs      # Trade detection
├── backtest/            # Backtesting
│   ├── engine.rs        # Backtest orchestration
│   ├── simulator.rs     # Trade simulation
│   ├── slippage.rs      # Slippage models
│   └── metrics.rs       # Performance metrics
└── storage/             # Persistence
    └── trade_log.rs     # Trade logging
```

## Roadmap

- [ ] WebSocket RTDS monitoring
- [ ] Advanced risk management (stop-loss, take-profit)
- [ ] Multi-trader portfolio optimization
- [ ] Real-time performance dashboard
- [ ] SQLite/PostgreSQL integration
- [ ] Docker containerization

## Resources

- [Polymarket Documentation](https://docs.polymarket.com)
- [CLOB API Reference](https://docs.polymarket.com/developers/CLOB/introduction)
- [EIP-712 Specification](https://eips.ethereum.org/EIPS/eip-712)

## License

MIT License - See [LICENSE](LICENSE) for details

## Disclaimer

This software is for educational and research purposes. Use at your own risk. The authors are not responsible for any financial losses incurred through use of this software. Always test thoroughly before deploying real capital.

---

**Questions?** Open an issue or check the documentation in `implementation_outline.md` and `quick_reference.md`.
