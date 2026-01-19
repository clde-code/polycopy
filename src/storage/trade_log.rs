use crate::errors::Result;
use crate::models::{ExecutedTrade, Trade};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeLogEntry {
    pub timestamp: String,
    pub trade: Trade,
    pub executed: Option<ExecutedTrade>,
    pub success: bool,
    pub error: Option<String>,
}

pub struct TradeLogger {
    log_path: String,
}

impl TradeLogger {
    pub fn new(log_path: String) -> Self {
        Self { log_path }
    }

    /// Log a detected trade
    pub fn log_detected_trade(&self, trade: &Trade) -> Result<()> {
        let entry = TradeLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            trade: trade.clone(),
            executed: None,
            success: false,
            error: None,
        };

        self.write_entry(&entry)
    }

    /// Log a successfully executed trade
    pub fn log_executed_trade(&self, trade: &Trade, executed: &ExecutedTrade) -> Result<()> {
        let entry = TradeLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            trade: trade.clone(),
            executed: Some(executed.clone()),
            success: true,
            error: None,
        };

        self.write_entry(&entry)
    }

    /// Log a failed trade execution
    pub fn log_failed_trade(&self, trade: &Trade, error: &str) -> Result<()> {
        let entry = TradeLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            trade: trade.clone(),
            executed: None,
            success: false,
            error: Some(error.to_string()),
        };

        self.write_entry(&entry)
    }

    /// Write an entry to the log file
    fn write_entry(&self, entry: &TradeLogEntry) -> Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string(entry)?;
        writeln!(writer, "{}", json)?;
        writer.flush()?;

        Ok(())
    }

    /// Read all log entries
    pub fn read_logs(&self) -> Result<Vec<TradeLogEntry>> {
        if !Path::new(&self.log_path).exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        use std::io::BufRead;
        for line in reader.lines() {
            if let Ok(line) = line {
                if let Ok(entry) = serde_json::from_str::<TradeLogEntry>(&line) {
                    entries.push(entry);
                }
            }
        }

        Ok(entries)
    }

    /// Get trade statistics from logs
    pub fn get_statistics(&self) -> Result<TradeStatistics> {
        let entries = self.read_logs()?;

        let total_trades = entries.len();
        let successful_trades = entries.iter().filter(|e| e.success).count();
        let failed_trades = total_trades - successful_trades;

        Ok(TradeStatistics {
            total_trades,
            successful_trades,
            failed_trades,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeStatistics {
    pub total_trades: usize,
    pub successful_trades: usize,
    pub failed_trades: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::OrderSide;
    use chrono::Utc;
    use rust_decimal_macros::dec;
    use std::fs;

    #[test]
    fn test_trade_logger() {
        let log_path = "/tmp/test_trade_log.jsonl";
        let _ = fs::remove_file(log_path); // Clean up from previous test

        let logger = TradeLogger::new(log_path.to_string());

        let trade = Trade {
            id: "test1".to_string(),
            market_id: "market1".to_string(),
            trader: "0x0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
            side: OrderSide::Buy,
            price: dec!(0.5),
            size: dec!(100),
            size_usdc: dec!(50),
            timestamp: Utc::now(),
            trader_win_rate: None,
        };

        logger.log_detected_trade(&trade).unwrap();

        let logs = logger.read_logs().unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].trade.id, "test1");

        // Clean up
        let _ = fs::remove_file(log_path);
    }
}
