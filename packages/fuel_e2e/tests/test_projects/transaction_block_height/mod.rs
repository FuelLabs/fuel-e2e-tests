use fuels::prelude::*;

use fuels::prelude::Error::TransactionError;
use test_abigen::test_project_abigen;
use test_macros::test_project_bin_path;

#[tokio::test]
async fn contract_deployment_respects_maturity() -> anyhow::Result<()> {
    test_project_abigen!(MyContract, "transaction_block_height");

    let wallet = launch_provider_and_get_wallet().await;

    let deploy_w_maturity = |maturity| {
        let parameters = TxParameters {
            maturity,
            ..TxParameters::default()
        };
        Contract::deploy(
            test_project_bin_path!("transaction_block_height"),
            &wallet,
            parameters,
            StorageConfiguration::default(),
        )
    };

    let err = deploy_w_maturity(1).await.expect_err("Should not have been able to deploy the contract since the block height (0) is less than the requested maturity (1)");
    assert!(matches!(err, TransactionError(msg) if msg.contains("TransactionMaturity")));

    produce_blocks(&wallet, 1).await?;
    deploy_w_maturity(1)
        .await
        .expect("Should be able to deploy now since maturity (1) is <= than the block height (1)");

    Ok(())
}
#[tokio::test]
async fn contract_method_call_respects_maturity() -> anyhow::Result<()> {
    test_project_abigen!(MyContract, "transaction_block_height");

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("transaction_block_height"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = MyContractBuilder::new(id.to_string(), wallet.clone()).build();

    let call_w_maturity = |call_maturity| {
        let mut prepared_call = instance.calling_this_will_produce_a_block();
        prepared_call.tx_parameters.maturity = call_maturity;
        prepared_call.call()
    };

    call_w_maturity(1).await.expect("Should have passed since we're calling with a maturity that is less or equal to the current block height");

    call_w_maturity(3).await.expect_err("Should have failed since we're calling with a maturity that is greater than the current block height");

    Ok(())
}
#[tokio::test]
async fn gql_height_info_is_correct() -> anyhow::Result<()> {
    test_project_abigen!(MyContract, "transaction_block_height");

    let wallet = launch_provider_and_get_wallet().await;
    let provider = &wallet.get_provider().unwrap();

    let id = Contract::deploy(
        test_project_bin_path!("transaction_block_height"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    let instance = MyContractBuilder::new(id.to_string(), wallet.clone()).build();

    let block_height_from_contract = || async {
        Ok(instance.get_current_height().simulate().await?.value) as Result<u64, Error>
    };

    assert_eq!(provider.latest_block_height().await?, 1);
    assert_eq!(block_height_from_contract().await?, 1);

    produce_blocks(&wallet, 3).await?;

    assert_eq!(provider.latest_block_height().await?, 4);
    assert_eq!(block_height_from_contract().await?, 4);
    Ok(())
}
