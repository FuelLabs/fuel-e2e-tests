use fuels::prelude::*;
use test_abigen::test_project_abigen;
use test_macros::test_project_bin_path;

#[tokio::test]
async fn call_with_empty_return() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    test_project_abigen!(MyContract, "call_empty_return");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("call_empty_return"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();

    let _response = contract_instance
        .store_value(42) // Build the ABI call
        .call() // Perform the network call
        .await?;
    Ok(())
}
