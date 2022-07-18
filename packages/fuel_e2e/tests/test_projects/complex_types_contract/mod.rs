use fuels::prelude::*;
use test_abigen::test_project_abigen;
use test_macros::test_project_bin_path;

#[tokio::test]
async fn call_with_structs() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    test_project_abigen!(MyContract, "complex_types_contract");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("complex_types_contract"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();
    let counter_config = CounterConfig {
        dummy: true,
        initial_value: 42,
    };

    let response = contract_instance
        .initialize_counter(counter_config) // Build the ABI call
        .call() // Perform the network call
        .await?;

    assert_eq!(42, response.value);

    let response = contract_instance.increment_counter(10).call().await?;

    assert_eq!(52, response.value);
    Ok(())
}
