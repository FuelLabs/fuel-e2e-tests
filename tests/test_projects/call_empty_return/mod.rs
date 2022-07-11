use fuels::prelude::*;

#[tokio::test]
async fn call_with_empty_return() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    abigen!(
        MyContract,
        "tests/test_projects/call_empty_return/out/debug/contract_test-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/call_empty_return/out/debug/contract_test.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

    let _response = contract_instance
        .store_value(42) // Build the ABI call
        .call() // Perform the network call
        .await?;
    Ok(())
}
