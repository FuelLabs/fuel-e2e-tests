use fuel_e2e_tests::setup::{self, Setup};
use fuels::{prelude::*, programs::executable::Executable, types::output::Output};

// #[tokio::test]
// async fn pay_with_predicate -> color_eyre::Result<()> {
//     abigen!(
//         Contract(
//             name = "MyContract",
//             abi = "sway/contract_test/out/release/contract_test-abi.json"
//         ),
//         Predicate(
//             name = "MyPredicate",
//         abi = "sway/predicate_blobs/out/release/predicate_blobs-abi.json"
//         )
//     );

//     let predicate_data = MyPredicateEncoder::default().encode_data(32768)?;

//     let mut predicate: Predicate =
//         Predicate::load_from("sway/types/predicates/u64/out/release/u64.bin")?
//             .with_data(predicate_data);

//     let num_coins = 4;
//     let num_messages = 8;
//     let amount = 16;
//     let (provider, _predicate_balance, _receiver, _receiver_balance, _asset_id, _) =
//         setup_predicate_test(predicate.address(), num_coins, num_messages, amount).await?;

//     predicate.set_provider(provider.clone());

//     let contract_id = Contract::load_from(
//         "sway/contracts/contract_test/out/release/contract_test.bin",
//         LoadConfiguration::default(),
//     )?
//     .deploy_if_not_exists(&predicate, TxPolicies::default())
//     .await?;

//     let contract_methods = MyContract::new(contract_id.clone(), predicate.clone()).methods();
//     let tx_policies = TxPolicies::default()
//         .with_tip(1)
//         .with_script_gas_limit(1_000_000);

//     // TODO: https://github.com/FuelLabs/fuels-rs/issues/1394
//     let expected_fee = 1;
//     let consensus_parameters = provider.consensus_parameters().await?;
//     assert_eq!(
//         predicate
//             .get_asset_balance(consensus_parameters.base_asset_id())
//             .await?,
//         192 - expected_fee
//     );

//     let response = contract_methods
//         .initialize_counter(42) // Build the ABI call
//         .with_tx_policies(tx_policies)
//         .call()
//         .await?;

//     assert_eq!(42, response.value);
//     // TODO: https://github.com/FuelLabs/fuels-rs/issues/1394
//     let expected_fee = 2;
//     assert_eq!(
//         predicate
//             .get_asset_balance(consensus_parameters.base_asset_id())
//             .await?,
//         191 - expected_fee
//     );

//     Ok(())
// }

#[tokio::test]
async fn predicate_blobs() -> color_eyre::Result<()> {
    abigen!(Predicate(
        name = "MyPredicate",
        abi = "sway/predicate_blobs/out/release/predicate_blobs-abi.json"
    ));

    let configurables = MyPredicateConfigurables::default().with_SECRET_NUMBER(10001)?;

    let predicate_data = MyPredicateEncoder::default().encode_data(1, 19)?;

    let executable = Executable::load_from("sway/predicate_blobs/out/release/predicate_blobs.bin")?;

    let loader = executable
        .convert_to_loader()?
        .with_configurables(configurables);

    let mut predicate: Predicate = Predicate::from_code(loader.code()).with_data(predicate_data);

    let Setup { wallet, .. } = setup::init().await?;
    let provider = wallet.try_provider()?.clone();
    predicate.set_provider(provider.clone());

    let base_asset_id = *provider
        .chain_info()
        .await?
        .consensus_parameters
        .base_asset_id();

    // fund predicate
    let amount = 5000;
    wallet
        .transfer(
            predicate.address(),
            amount,
            base_asset_id,
            TxPolicies::default(),
        )
        .await?;
    assert_eq!(predicate.get_asset_balance(&base_asset_id).await?, amount);

    loader.upload_blob(wallet.clone()).await?;

    // transfer all coins from predicate back to wallet
    let wallet_amount_before_return = wallet.get_asset_balance(&base_asset_id).await?;
    transfer_all(&predicate, wallet.address(), base_asset_id).await?;
    assert_eq!(predicate.get_asset_balance(&base_asset_id).await?, 0);

    let wallet_amount_after_return = wallet.get_asset_balance(&base_asset_id).await?;

    assert!(wallet_amount_after_return > wallet_amount_before_return);

    Ok(())
}

async fn transfer_all(
    account: &impl Account,
    to: &Bech32Address,
    asset_id: AssetId,
) -> color_eyre::Result<()> {
    let provider = account.try_provider()?;
    let account_balance = account.get_asset_balance(&asset_id).await?;

    let inputs = account
        .get_asset_inputs_for_amount(asset_id, account_balance, None)
        .await?;
    let outputs = vec![Output::change(to.into(), 0, asset_id)];

    let tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());

    let tx = tb.build(provider).await?;

    provider.send_transaction_and_await_commit(tx).await?;

    Ok(())
}
