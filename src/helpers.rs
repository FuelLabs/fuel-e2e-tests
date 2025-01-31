use color_eyre::eyre::eyre;
use fuel_core_client::client::types::TransactionStatus;
use fuels::{accounts::provider::Provider, client::FuelClient, types::Bytes32};

pub trait ProviderExt {
    async fn get_tx_total_fee(&self, id: &Bytes32) -> color_eyre::Result<Option<u64>>;
}

impl ProviderExt for Provider {
    async fn get_tx_total_fee(&self, id: &Bytes32) -> color_eyre::Result<Option<u64>> {
        let client = FuelClient::new(self.url()).map_err(|e| eyre!(e.to_string()))?;

        let tx = client.transaction(id).await?.unwrap();

        let fee = match tx.status {
            TransactionStatus::Success { total_fee, .. }
            | TransactionStatus::Failure { total_fee, .. } => Some(total_fee),
            _ => None,
        };

        Ok(fee)
    }
}
