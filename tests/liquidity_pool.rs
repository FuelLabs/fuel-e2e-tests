fuel_e2e_tests::define_fuels!();

use fuel_e2e_tests::setup::{self, Setup};
use utils::{DepositCompleted, DepositEvent, Fixture};

#[tokio::test]
async fn liquidity_pool() -> color_eyre::Result<()> {
    let Setup {
        wallet,
        deploy_config,
    } = setup::init().await?;

    let fixture = Fixture::deploy(&wallet, deploy_config).await?;

    // so that we don't lose funds in cases when the test failed/was killed before we reclaimed the deposit
    fixture.reclaim_any_previous_deposits().await?;

    let deposit_amount = 100;

    let pre_deposit_balances = fixture.current_balances().await?;
    let pre_deposit_total = fixture.total_deposited_ever().await?;

    let DepositCompleted { total_fee, event } = fixture.deposit(deposit_amount).await?;

    let post_deposit_total = fixture.total_deposited_ever().await?;
    let post_deposid_balances = fixture.current_balances().await?;

    // contract configured to mint 2x the amount deposited
    let amount_minted = deposit_amount * 2;

    assert_eq!(
        event,
        DepositEvent {
            amount: deposit_amount,
            minted: amount_minted,
            to: crate::fuels::accounts::ViewOnlyAccount::address(&wallet).into()
        }
    );

    assert_eq!(post_deposit_total, pre_deposit_total + deposit_amount);

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
    use fuel_e2e_tests::{
        helpers::{self},
        setup::DeployConfig,
    };
    use fuels::{prelude::*, types::Bits256};

    abigen!(Contract(
        name = "LiquidityContractBindings",
        abi = "sway/liquidity_pool/out/release/liquidity_pool-abi.json"
    ));

    pub struct Fixture {
        instance: LiquidityContractBindings<crate::setup::Wallet>,
    }

    #[derive(Debug)]
    pub struct DepositCompleted {
        pub total_fee: u64,
        pub event: DepositEvent,
    }

    impl Fixture {
        pub async fn reclaim_any_previous_deposits(&self) -> Result<()> {
            let balances = self.current_balances().await?;

            if balances.minted > 0 {
                self.withdraw(balances.minted).await?;
            }

            Ok(())
        }

        pub async fn deploy(
            wallet: &crate::setup::Wallet,
            deploy_config: DeployConfig,
        ) -> Result<Self> {
            let contract_id = helpers::deploy(
                wallet,
                deploy_config,
                "sway/liquidity_pool/out/release/liquidity_pool.bin",
            )
            .await?;

            let instance = LiquidityContractBindings::new(contract_id, wallet.clone());

            Ok(Self { instance })
        }

        pub async fn deposit(&self, amount: u64) -> Result<DepositCompleted> {
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

            let event = resp
                .decode_logs_with_type::<DepositEvent>()?
                .pop()
                .expect("should have had an event");

            #[cfg(feature = "fuels_lts_70")]
            let total_fee = helpers::ProviderExt::get_tx_total_fee(
                &self.provider(),
                &resp.tx_id.expect("should have tx_id"),
            )
            .await?
            .expect("tx executed");

            #[cfg(feature = "fuels_71")]
            let total_fee = resp.tx_status.total_fee;

            Ok(DepositCompleted { total_fee, event })
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

            #[cfg(feature = "fuels_lts_70")]
            let total_fee = helpers::ProviderExt::get_tx_total_fee(
                &self.provider(),
                &resp.tx_id.expect("should have tx_id"),
            )
            .await?
            .expect("tx executed");

            #[cfg(feature = "fuels_71")]
            let total_fee = resp.tx_status.total_fee;

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
            self.instance.account().try_provider().unwrap().clone()
        }

        fn minted_asset_id(&self) -> AssetId {
            self.instance.contract_id().asset_id(&Bits256::zeroed())
        }

        pub async fn total_deposited_ever(&self) -> Result<u64> {
            let total_deposited = self
                .instance
                .methods()
                .total_deposited_ever()
                .simulate(Execution::StateReadOnly)
                .await?
                .value;

            Ok(total_deposited)
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
