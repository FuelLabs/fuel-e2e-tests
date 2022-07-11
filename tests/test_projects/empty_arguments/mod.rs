use fuels::prelude::*;

#[tokio::test]
async fn test_methods_typeless_argument() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    abigen!(
        MyContract,
        "tests/test_projects/empty_arguments/out/debug/method_four_arguments-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/empty_arguments/out/debug/method_four_arguments.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

    let response = contract_instance
        .method_with_empty_argument()
        .call()
        .await?;
    assert_eq!(response.value, 63);
    Ok(())
}
