use fuels::prelude::*;
use third::test_project_bin_path;

#[tokio::test]
async fn test_methods_typeless_argument() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    abigen!(
        MyContract,
        "packages/fuel_e2e/tests/test_projects/method_four_arguments/out/debug/method_four_arguments-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("method_four_arguments"),
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
