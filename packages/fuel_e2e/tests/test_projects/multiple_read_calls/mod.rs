use fuels::prelude::*;
use some_macros::test_project_abigen;
use third::test_project_bin_path;

#[tokio::test]
async fn multiple_read_calls() -> Result<(), Error> {
    test_project_abigen!(MyContract, "multiple_read_calls");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("multiple_read_calls"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    println!("Contract deployed @ {:x}", contract_id);
    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

    contract_instance.store(42).call().await?;

    // Use "simulate" because the methods don't actually run a transaction, but just a dry-run
    // We can notice here that, thanks to this, we don't generate a TransactionId collision,
    // even if the transactions are theoretically the same.
    let stored = contract_instance.read(0).simulate().await?;

    assert_eq!(stored.value, 42);

    let stored = contract_instance.read(0).simulate().await?;

    assert_eq!(stored.value, 42);
    Ok(())
}
