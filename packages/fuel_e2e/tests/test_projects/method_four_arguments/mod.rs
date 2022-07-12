use fuels::prelude::*;
use some_macros::test_project_abigen;
use third::test_project_bin_path;

#[tokio::test]
async fn test_methods_typeless_argument() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    test_project_abigen!(MyContract, "method_four_arguments");

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
