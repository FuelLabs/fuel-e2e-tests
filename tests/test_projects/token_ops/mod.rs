use fuels::prelude::*;
use fuels::tx::Receipt;

#[tokio::test]
async fn test_amount_and_asset_forwarding() -> Result<(), Error> {
    abigen!(
        TestFuelCoinContract,
        "tests/test_projects/token_ops/out/debug/token_ops-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/token_ops/out/debug/token_ops.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = TestFuelCoinContract::new(id.to_string(), wallet.clone());

    let mut balance_response = instance.get_balance(id, id).call().await?;
    assert_eq!(balance_response.value, 0);

    instance.mint_coins(5_000_000).call().await?;

    balance_response = instance.get_balance(id, id).call().await?;
    assert_eq!(balance_response.value, 5_000_000);

    let tx_params = TxParameters::new(None, Some(1_000_000), None, None);
    // Forward 1_000_000 coin amount of base asset_id
    // this is a big number for checking that amount can be a u64
    let call_params = CallParameters::new(Some(1_000_000), None, None);

    let response = instance
        .get_msg_amount()
        .tx_params(tx_params)
        .call_params(call_params)
        .call()
        .await?;

    assert_eq!(response.value, 1_000_000);

    let call_response = response
        .receipts
        .iter()
        .find(|&r| matches!(r, Receipt::Call { .. }));

    assert!(call_response.is_some());

    assert_eq!(call_response.unwrap().amount().unwrap(), 1_000_000);
    assert_eq!(call_response.unwrap().asset_id().unwrap(), &BASE_ASSET_ID);

    let address = wallet.address();

    // withdraw some tokens to wallet
    instance
        .transfer_coins_to_output(1_000_000, id, address)
        .append_variable_outputs(1)
        .call()
        .await?;

    let call_params = CallParameters::new(Some(0), Some(AssetId::from(*id)), None);
    let tx_params = TxParameters::new(None, Some(1_000_000), None, None);

    let response = instance
        .get_msg_amount()
        .tx_params(tx_params)
        .call_params(call_params)
        .call()
        .await?;

    assert_eq!(response.value, 0);

    let call_response = response
        .receipts
        .iter()
        .find(|&r| matches!(r, Receipt::Call { .. }));

    assert!(call_response.is_some());

    assert_eq!(call_response.unwrap().amount().unwrap(), 0);
    assert_eq!(
        call_response.unwrap().asset_id().unwrap(),
        &AssetId::from(*id)
    );
    Ok(())
}
