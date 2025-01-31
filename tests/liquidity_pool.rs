use fuel_e2e_tests::setup;
use utils::Fixture;

#[tokio::test]
async fn liquidity_pool() -> color_eyre::Result<()> {
    // when making tweaks to the test
    // let wallet = launch_provider_and_get_wallet().await?;
    let wallet = setup::init().await?;
    let fixture = Fixture::deploy_if_not_exists(&wallet).await?;

    let deposit_amount = 100;

    let pre_deposit_balances = fixture.current_balances().await?;

    let total_fee = fixture.deposit(deposit_amount).await?;

    let post_deposid_balances = fixture.current_balances().await?;
    let amount_minted = deposit_amount * 2;

    assert_eq!(
        post_deposid_balances.base,
        pre_deposit_balances.base - deposit_amount - total_fee
    );

    assert_eq!(
        post_deposid_balances.minted,
        pre_deposit_balances.minted + amount_minted
    );

    let pre_withdraw_balance = fixture.current_balances().await?;
    let total_fee = fixture.withdraw(amount_minted).await?;
    let post_withdraw_balance = fixture.current_balances().await?;

    assert_eq!(
        post_withdraw_balance.base,
        pre_withdraw_balance.base + deposit_amount - total_fee
    );

    Ok(())
}

mod utils {
    use color_eyre::Result;
    use fuel_e2e_tests::helpers::ProviderExt;
    use fuels::{prelude::*, types::Bits256};

    abigen!(Contract(
        name = "LiquidityContractBindings",
        abi = "sway/liquidity_pool/out/release/liquidity_pool-abi.json"
    ));

    pub struct Fixture {
        instance: LiquidityContractBindings<WalletUnlocked>,
    }

    impl Fixture {
        pub async fn deploy_if_not_exists(wallet: &WalletUnlocked) -> Result<Self> {
            let contract_id = Contract::load_from(
                "sway/liquidity_pool/out/release/liquidity_pool.bin",
                LoadConfiguration::default(),
            )
            .unwrap()
            .deploy_if_not_exists(wallet, TxPolicies::default())
            .await?;

            let instance = LiquidityContractBindings::new(contract_id, wallet.clone());

            Ok(Self { instance })
        }

        pub async fn deposit(&self, amount: u64) -> Result<u64> {
            let call_params = CallParameters::default()
                .with_amount(amount)
                .with_asset_id(self.base_asset_id().await?);

            let resp = self
                .instance
                .methods()
                .deposit(self.instance.account().address().into())
                .call_params(call_params)?
                .with_variable_output_policy(VariableOutputPolicy::Exactly(1))
                .call()
                .await?;

            let total_fee = self
                .provider()
                .get_tx_total_fee(&resp.tx_id.expect("should have tx_id"))
                .await?
                .expect("tx executed");

            Ok(total_fee)
        }

        pub async fn withdraw(&self, amount: u64) -> Result<u64> {
            let call_params = CallParameters::default()
                .with_amount(amount)
                .with_asset_id(self.minted_asset_id());

            let resp = self
                .instance
                .methods()
                .withdraw(self.instance.account().address().into())
                .call_params(call_params)?
                .with_variable_output_policy(VariableOutputPolicy::Exactly(1))
                .call()
                .await?;

            let total_fee = self
                .provider()
                .get_tx_total_fee(&resp.tx_id.expect("should have tx_id"))
                .await?
                .expect("tx executed");

            Ok(total_fee)
        }

        pub async fn base_asset_id(&self) -> Result<AssetId> {
            Ok(*self
                .provider()
                .chain_info()
                .await?
                .consensus_parameters
                .base_asset_id())
        }

        fn provider(&self) -> Provider {
            self.instance.account().provider().unwrap().clone()
        }

        fn minted_asset_id(&self) -> AssetId {
            self.instance.contract_id().asset_id(&Bits256::zeroed())
        }

        pub async fn current_balances(&self) -> Result<Balances> {
            let base_asset_id = self.base_asset_id().await?;
            let base_balance = self
                .instance
                .account()
                .get_asset_balance(&base_asset_id)
                .await?;

            let minted_asset_id = self.minted_asset_id();
            let minted_balance = self
                .instance
                .account()
                .get_asset_balance(&minted_asset_id)
                .await?;

            Ok(Balances {
                base: base_balance,
                minted: minted_balance,
            })
        }
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct Balances {
        pub base: u64,
        pub minted: u64,
    }
}
