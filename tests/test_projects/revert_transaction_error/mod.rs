use fuels::prelude::*;

#[tokio::test]
async fn test_reverting_transaction() -> Result<(), Error> {
    abigen!(
        RevertingContract,
        "tests/test_projects/revert_transaction_error/out/debug/capture_revert_transaction_error-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/revert_transaction_error/out/debug/capture_revert_transaction_error.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
        .await?;
    let contract_instance = RevertingContract::new(contract_id.to_string(), wallet);
    println!("Contract deployed @ {:x}", contract_id);
    let response = contract_instance.make_transaction_fail(0).call().await;
    assert!(matches!(response, Err(Error::ContractCallError(..))));
    Ok(())
}
