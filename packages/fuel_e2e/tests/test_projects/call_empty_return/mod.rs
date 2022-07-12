use fuels::prelude::*;
use third::test_project_bin_path;

#[tokio::test]
async fn call_with_empty_return() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    abigen!(
        MyContract,
        "packages/fuel_e2e/tests/test_projects/call_empty_return/out/debug/call_empty_return-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("call_empty_return"),
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
