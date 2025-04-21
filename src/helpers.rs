#[cfg(feature = "fuels_lts_70")]
mod fuels_lts_70_overrides {
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
    use rand::Rng;

    use crate::setup::DeployConfig;

    #[allow(async_fn_in_trait)]
    pub trait ProviderExt {
        async fn get_tx_total_fee(&self, id: &Bytes32) -> Result<Option<u64>>;
    }

    impl ProviderExt for Provider {
        async fn get_tx_total_fee(&self, id: &fuels_lts_70::types::Bytes32) -> Result<Option<u64>> {
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
    pub async fn deploy(
        wallet: &WalletUnlocked,
        deploy_config: DeployConfig,
        contract_bin: &str,
    ) -> color_eyre::Result<Bech32ContractId> {
        let salt: [u8; 32] = if deploy_config.force_deploy {
            rand::rng().random()
        } else {
            [0; 32]
        };

        let contract_id = if deploy_config.deploy_in_blobs {
            deploy_blobbed(contract_bin, wallet, salt).await?
        } else {
            deploy_normal(contract_bin, wallet, salt).await?
        };

        Ok(contract_id)
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
}
#[cfg(feature = "fuels_lts_70")]
pub use fuels_lts_70_overrides::*;

#[cfg(feature = "fuels_71")]
mod fuels_71_overrides {
    use std::cmp::min;

    use color_eyre::Result;
    use fuels::{
        accounts::wallet::Wallet,
        core::constants::WORD_SIZE,
        programs::contract::{Contract, LoadConfiguration},
        types::{bech32::Bech32ContractId, transaction::TxPolicies},
    };
    use rand::Rng;

    use crate::setup::DeployConfig;

    pub async fn deploy(
        wallet: &Wallet,
        deploy_config: DeployConfig,
        contract_bin: &str,
    ) -> color_eyre::Result<Bech32ContractId> {
        let salt: [u8; 32] = if deploy_config.force_deploy {
            rand::rng().random()
        } else {
            [0; 32]
        };

        let contract_id = if deploy_config.deploy_in_blobs {
            deploy_blobbed(contract_bin, wallet, salt).await?
        } else {
            deploy_normal(contract_bin, wallet, salt).await?
        };

        Ok(contract_id)
    }

    pub async fn deploy_blobbed(
        contract_bin: &str,
        wallet: &Wallet,
        salt: [u8; 32],
    ) -> Result<Bech32ContractId> {
        let contract_size = std::fs::metadata(contract_bin)?.len();
        let max_contract_size = wallet
            .provider()
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
                .await?
                .contract_id;
        Ok(bech32_contract_id)
    }

    pub async fn deploy_normal(
        contract_bin: &str,
        wallet: &Wallet,
        salt: [u8; 32],
    ) -> Result<Bech32ContractId> {
        let bech32_contract_id =
            Contract::load_from(contract_bin, LoadConfiguration::default().with_salt(salt))?
                .deploy_if_not_exists(wallet, TxPolicies::default())
                .await?
                .contract_id;
        Ok(bech32_contract_id)
    }
}
#[cfg(feature = "fuels_71")]
pub use fuels_71_overrides::*;
