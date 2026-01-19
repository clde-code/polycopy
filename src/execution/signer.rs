use crate::errors::{PolymarketError, Result};
use crate::models::Order;
use ethers::signers::{LocalWallet, Signer};
use ethers::types::{Address, H256};
use std::sync::Arc;

pub struct OrderSigner {
    wallet: Arc<LocalWallet>,
    chain_id: u64,
}

impl OrderSigner {
    /// Create a new order signer from a private key
    pub fn new(private_key: &str, chain_id: u64) -> Result<Self> {
        let wallet = private_key
            .parse::<LocalWallet>()
            .map_err(|e| PolymarketError::SigningError(format!("Invalid private key: {}", e)))?;

        Ok(Self {
            wallet: Arc::new(wallet),
            chain_id,
        })
    }

    /// Get the wallet address
    pub fn address(&self) -> Address {
        self.wallet.address()
    }

    /// Sign authentication message for API access (EIP-712)
    pub async fn sign_auth_message(&self, timestamp: u64, nonce: u64) -> Result<String> {
        let message = format!(
            "This message attests that I control the given wallet\nTimestamp: {}\nNonce: {}",
            timestamp, nonce
        );

        let signature = self
            .wallet
            .sign_message(message.as_bytes())
            .await
            .map_err(|e| PolymarketError::SigningError(format!("Failed to sign message: {}", e)))?;

        Ok(format!("0x{}", hex::encode(signature.to_vec())))
    }

    /// Sign an order using EIP-712 structured data hashing
    pub async fn sign_order(&self, order: &Order) -> Result<String> {
        // Note: This is a simplified version - actual implementation would match Polymarket's exact EIP-712 schema
        let order_hash = self.hash_order(order)?;

        let signature = self
            .wallet
            .sign_hash(order_hash)
            .map_err(|e| PolymarketError::SigningError(format!("Failed to sign order: {}", e)))?;

        Ok(format!("0x{}", hex::encode(signature.to_vec())))
    }

    /// Hash the order data according to EIP-712
    fn hash_order(&self, order: &Order) -> Result<H256> {
        // This is a simplified implementation
        // In production, this would need to match Polymarket's exact EIP-712 schema
        use ethers::utils::keccak256;

        let mut data = Vec::new();
        data.extend_from_slice(order.market_id.as_bytes());
        data.extend_from_slice(&order.price_decimal.to_string().as_bytes());
        data.extend_from_slice(&order.quantity.to_string().as_bytes());
        data.extend_from_slice(&[match order.side {
            crate::models::OrderSide::Buy => 0u8,
            crate::models::OrderSide::Sell => 1u8,
        }]);
        data.extend_from_slice(order.owner.as_bytes());
        data.extend_from_slice(&order.expiration_time.to_le_bytes());

        Ok(H256::from_slice(&keccak256(&data)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    const TEST_PRIVATE_KEY: &str =
        "0x0123456789012345678901234567890123456789012345678901234567890123";

    #[tokio::test]
    async fn test_signer_creation() {
        let signer = OrderSigner::new(TEST_PRIVATE_KEY, 137);
        assert!(signer.is_ok());
    }

    #[tokio::test]
    async fn test_sign_auth_message() {
        let signer = OrderSigner::new(TEST_PRIVATE_KEY, 137).unwrap();
        let signature = signer.sign_auth_message(1234567890, 0).await;
        assert!(signature.is_ok());
        assert!(signature.unwrap().starts_with("0x"));
    }

    #[tokio::test]
    async fn test_sign_order() {
        let signer = OrderSigner::new(TEST_PRIVATE_KEY, 137).unwrap();
        let order = Order {
            market_id: "test_market".to_string(),
            price_decimal: Decimal::new(5, 1), // 0.5
            quantity: Decimal::new(100, 0),
            side: crate::models::OrderSide::Buy,
            owner: signer.address(),
            expiration_time: 1234567890,
        };

        let signature = signer.sign_order(&order).await;
        assert!(signature.is_ok());
        assert!(signature.unwrap().starts_with("0x"));
    }
}
