use fuel_e2e_tests::{
    helpers,
    setup::{self, Setup},
};
use fuels::{prelude::*, programs::executable::Executable, types::output::Output};

#[tokio::test]
async fn pay_contract_call_with_predicate() -> color_eyre::Result<()> {
    abigen!(
        Contract(
            name = "MyContract",
            abi = "sway/contract_test/out/release/contract_test-abi.json"
        ),
        Predicate(
            name = "MyPredicate",
            abi = "sway/predicate_blobs/out/release/predicate_blobs-abi.json"
        )
    );

    let predicate_data = MyPredicateEncoder::default().encode_data(1, 19)?;
    let configurables = MyPredicateConfigurables::default().with_SECRET_NUMBER(10001)?;

    let mut predicate: Predicate =
        Predicate::load_from("sway/predicate_blobs/out/release/predicate_blobs.bin")?
            .with_data(predicate_data)
            .with_configurables(configurables);

    let Setup {
        wallet,
        deploy_config,
    } = setup::init().await?;
    let provider = wallet.try_provider()?.clone();
    predicate.set_provider(provider.clone());

    let base_asset_id = *provider
        .chain_info()
        .await?
        .consensus_parameters
        .base_asset_id();

    // empty out predicate if it has any coins left
    maybe_transfer_all(&predicate, &wallet, base_asset_id).await?;

    // fund predicate
    let amount = 10_000;
    wallet
        .transfer(
            predicate.address(),
            amount,
            base_asset_id,
            TxPolicies::default(),
        )
        .await?;
    assert_eq!(predicate.get_asset_balance(&base_asset_id).await?, amount);

    let contract_id = helpers::deploy(
        &wallet,
        deploy_config,
        "sway/liquidity_pool/out/release/liquidity_pool.bin",
    )
    .await?;

    // call contract method with predicate
    let response = MyContract::new(contract_id.clone(), predicate.clone())
        .methods()
        .initialize_counter(42)
        .call()
        .await?;

    assert_eq!(42, response.value);

    // transfer all coins from predicate back to wallet
    let wallet_amount_before_return = wallet.get_asset_balance(&base_asset_id).await?;
    maybe_transfer_all(&predicate, &wallet, base_asset_id).await?;
    assert_eq!(predicate.get_asset_balance(&base_asset_id).await?, 0);

    let wallet_amount_after_return = wallet.get_asset_balance(&base_asset_id).await?;
    assert!(wallet_amount_after_return > wallet_amount_before_return);

    Ok(())
}

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

    // empty out predicate if it has any coins left
    maybe_transfer_all(&predicate, &wallet, base_asset_id).await?;

    // fund predicate
    let amount = 10_000;
    wallet
        .transfer(
            predicate.address(),
            amount,
            base_asset_id,
            TxPolicies::default(),
        )
        .await?;
    // assert_eq!(predicate.get_asset_balance(&base_asset_id).await?, amount);

    loader.upload_blob(wallet.clone()).await?;

    // transfer all coins from predicate back to wallet
    let wallet_amount_before_return = wallet.get_asset_balance(&base_asset_id).await?;
    maybe_transfer_all(&predicate, &wallet, base_asset_id).await?;
    assert_eq!(predicate.get_asset_balance(&base_asset_id).await?, 0);

    let wallet_amount_after_return = wallet.get_asset_balance(&base_asset_id).await?;
    assert!(wallet_amount_after_return > wallet_amount_before_return);

    Ok(())
}

async fn maybe_transfer_all(
    from: &impl Account,
    funder_and_receiver: &WalletUnlocked,
    asset_id: AssetId,
) -> color_eyre::Result<()> {
    let provider = from.try_provider()?;
    let account_balance = from.get_asset_balance(&asset_id).await?;

    if account_balance == 0 {
        return Ok(());
    }

    let inputs = from
        .get_asset_inputs_for_amount(asset_id, account_balance, None)
        .await?;
    let outputs = vec![Output::change(
        funder_and_receiver.address().into(),
        0,
        asset_id,
    )];

    let mut tb = ScriptTransactionBuilder::prepare_transfer(inputs, outputs, TxPolicies::default());
    funder_and_receiver
        .adjust_for_fee(&mut tb, account_balance)
        .await?;
    tb.add_signer(funder_and_receiver.clone())?;

    let tx = tb.build(provider).await?;

    provider.send_transaction_and_await_commit(tx).await?;

    Ok(())
}
