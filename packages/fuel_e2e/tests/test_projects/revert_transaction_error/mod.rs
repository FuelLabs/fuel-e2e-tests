use fuels::prelude::*;
use some_macros::test_project_abigen;
use third::test_project_bin_path;

#[tokio::test]
async fn test_reverting_transaction() -> Result<(), Error> {
    test_project_abigen!(RevertingContract, "revert_transaction_error");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("revert_transaction_error"),
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
