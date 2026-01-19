use crate::errors::{PolymarketError, Result};
use crate::execution::signer::OrderSigner;
use crate::models::{MarketData, Order, OrderRequest, OrderResponse, OrderSide, OrderType};
use ethers::types::Address;
use reqwest::Client;
use rust_decimal::Decimal;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ClobClient {
    http_client: Client,
    api_url: String,
    signer: Arc<OrderSigner>,
    address: Address,
}

impl ClobClient {
    pub fn new(api_url: String, signer: OrderSigner) -> Self {
        let address = signer.address();
        Self {
            http_client: Client::new(),
            api_url,
            signer: Arc::new(signer),
            address,
        }
    }

    /// Place an order on the CLOB
    pub async fn place_order(
        &self,
        market_id: &str,
        side: OrderSide,
        price: Decimal,
        size: Decimal,
        order_type: OrderType,
    ) -> Result<OrderResponse> {
        // Get market tick size for price adjustment
        let tick_size = self.get_tick_size(market_id).await?;
        let adjusted_price = self.adjust_to_tick_size(price, tick_size);

        // Calculate expiration (10 minutes from now)
        let expiration_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 600;

        // Create order
        let order = Order {
            market_id: market_id.to_string(),
            price_decimal: adjusted_price,
            quantity: size,
            side: side.clone(),
            owner: self.address,
            expiration_time,
        };

        // Sign order
        let signature = self.signer.sign_order(&order).await?;

        // Get auth headers
        let (timestamp, nonce) = self.get_timestamp_and_nonce();
        let auth_signature = self.signer.sign_auth_message(timestamp, nonce).await?;

        // Create request
        let request = OrderRequest {
            order: order.clone(),
            owner: format!("{:?}", self.address),
            order_type: order_type.to_string(),
            post_only: false,
            fee_rate_bps: "0".to_string(),
            side: side.to_string(),
            signature_type: 0, // EOA
            signature,
        };

        // Send to API
        let response = self
            .http_client
            .post(&format!("{}/order", self.api_url))
            .header("POLY_ADDRESS", format!("{:?}", self.address))
            .header("POLY_SIGNATURE", &auth_signature)
            .header("POLY_TIMESTAMP", timestamp.to_string())
            .header("POLY_NONCE", nonce.to_string())
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(PolymarketError::ApiError(format!(
                "Failed to place order: {}",
                error
            )));
        }

        let order_response: OrderResponse = response.json().await?;
        Ok(order_response)
    }

    /// Get order status
    pub async fn get_order(&self, order_id: &str) -> Result<OrderResponse> {
        let (timestamp, nonce) = self.get_timestamp_and_nonce();
        let auth_signature = self.signer.sign_auth_message(timestamp, nonce).await?;

        let response = self
            .http_client
            .get(&format!("{}/order/{}", self.api_url, order_id))
            .header("POLY_ADDRESS", format!("{:?}", self.address))
            .header("POLY_SIGNATURE", &auth_signature)
            .header("POLY_TIMESTAMP", timestamp.to_string())
            .header("POLY_NONCE", nonce.to_string())
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(PolymarketError::ApiError(format!(
                "Failed to get order: {}",
                error
            )));
        }

        let order_response: OrderResponse = response.json().await?;
        Ok(order_response)
    }

    /// Cancel an order
    pub async fn cancel_order(&self, order_id: &str) -> Result<()> {
        let (timestamp, nonce) = self.get_timestamp_and_nonce();
        let auth_signature = self.signer.sign_auth_message(timestamp, nonce).await?;

        let response = self
            .http_client
            .delete(&format!("{}/order/{}", self.api_url, order_id))
            .header("POLY_ADDRESS", format!("{:?}", self.address))
            .header("POLY_SIGNATURE", &auth_signature)
            .header("POLY_TIMESTAMP", timestamp.to_string())
            .header("POLY_NONCE", nonce.to_string())
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(PolymarketError::ApiError(format!(
                "Failed to cancel order: {}",
                error
            )));
        }

        Ok(())
    }

    /// Get market data including tick size
    pub async fn get_tick_size(&self, market_id: &str) -> Result<Decimal> {
        let response = self
            .http_client
            .get(&format!("{}/markets/{}", self.api_url, market_id))
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(Decimal::new(1, 3)); // Default to 0.001
        }

        let market_data: MarketData = response.json().await.unwrap_or(MarketData {
            market_id: market_id.to_string(),
            tick_size: Decimal::new(1, 3),
            min_size: Decimal::ONE,
            max_size: Decimal::new(1000000, 0),
            description: None,
        });

        Ok(market_data.tick_size)
    }

    /// Adjust price to match tick size
    fn adjust_to_tick_size(&self, price: Decimal, tick_size: Decimal) -> Decimal {
        if tick_size == Decimal::ZERO {
            return price;
        }
        let ticks = (price / tick_size).round();
        ticks * tick_size
    }

    /// Get current timestamp and nonce for authentication
    fn get_timestamp_and_nonce(&self) -> (u64, u64) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let nonce = 0; // Nonce can be incremented if needed
        (timestamp, nonce)
    }

    /// Get current USDC balance (mock implementation)
    pub async fn get_balance(&self) -> Result<Decimal> {
        // In a real implementation, this would query the blockchain
        // For now, return a placeholder
        Ok(Decimal::new(10000, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_adjust_to_tick_size() {
        let signer = OrderSigner::new(
            "0x0123456789012345678901234567890123456789012345678901234567890123",
            137,
        )
        .unwrap();
        let client = ClobClient::new("http://localhost".to_string(), signer);

        // Tick size 0.01
        let adjusted = client.adjust_to_tick_size(dec!(0.567), dec!(0.01));
        assert_eq!(adjusted, dec!(0.57));

        // Tick size 0.001
        let adjusted = client.adjust_to_tick_size(dec!(0.5678), dec!(0.001));
        assert_eq!(adjusted, dec!(0.568));
    }
}
