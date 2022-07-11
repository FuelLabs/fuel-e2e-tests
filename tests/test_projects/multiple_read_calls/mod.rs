use fuels::prelude::*;

#[tokio::test]
async fn multiple_read_calls() -> Result<(), Error> {
    abigen!(
        MyContract,
        "tests/test_projects/multiple_read_calls/out/debug/demo-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/multiple_read_calls/out/debug/demo.bin",
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
