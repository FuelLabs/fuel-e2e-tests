use std::cmp::min;

use color_eyre::{eyre::eyre, Result};
use fuel_core_client::client::types::TransactionStatus;
use fuels::{
    accounts::{provider::Provider, wallet::WalletUnlocked},
    client::FuelClient,
    core::constants::WORD_SIZE,
    programs::contract::{Contract, LoadConfiguration},
    types::{bech32::Bech32ContractId, transaction::TxPolicies, Bytes32},
};

#[allow(async_fn_in_trait)]
pub trait ProviderExt {
    async fn get_tx_total_fee(&self, id: &Bytes32) -> Result<Option<u64>>;
}

impl ProviderExt for Provider {
    async fn get_tx_total_fee(&self, id: &Bytes32) -> Result<Option<u64>> {
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

pub async fn deploy_blobbed(
    contract_bin: &str,
    wallet: &WalletUnlocked,
    salt: [u8; 32],
) -> Result<Bech32ContractId> {
    let contract_size = std::fs::metadata(contract_bin)?.len();
    let max_contract_size = wallet
        .provider()
        .unwrap()
        .chain_info()
        .await?
        .consensus_parameters
        .contract_params()
        .contract_max_size();
    const NUM_BLOBS: u64 = 3;
    let blob_size = min(
        contract_size / WORD_SIZE as u64 / NUM_BLOBS,
        max_contract_size,
    );
    let bech32_contract_id =
        Contract::load_from(contract_bin, LoadConfiguration::default().with_salt(salt))?
            .convert_to_loader(blob_size as usize)?
            .deploy_if_not_exists(wallet, TxPolicies::default())
            .await;
    Ok(bech32_contract_id?)
}

pub async fn deploy_normal(
    contract_bin: &str,
    wallet: &WalletUnlocked,
    salt: [u8; 32],
) -> Result<Bech32ContractId> {
    let bech32_contract_id =
        Contract::load_from(contract_bin, LoadConfiguration::default().with_salt(salt))?
            .deploy_if_not_exists(wallet, TxPolicies::default())
            .await;
    Ok(bech32_contract_id?)
}
