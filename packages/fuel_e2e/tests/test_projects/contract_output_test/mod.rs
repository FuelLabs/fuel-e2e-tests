use fuels::prelude::*;
use some_macros::test_project_abigen;
use third::test_project_bin_path;

#[tokio::test]
async fn type_safe_output_values() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    test_project_abigen!(MyContract, "contract_output_test");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("contract_output_test"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

    // `response`'s type matches the return type of `is_event()`
    let response = contract_instance.is_even(10).call().await?;
    assert!(response.value);

    // `response`'s type matches the return type of `return_my_string()`
    let response = contract_instance
        .return_my_string("fuel".to_string())
        .call()
        .await?;

    assert_eq!(response.value, "fuel");

    let my_struct = MyStruct { foo: 10, bar: true };

    let response = contract_instance.return_my_struct(my_struct).call().await?;

    assert_eq!(response.value.foo, 10);
    assert!(response.value.bar);
    Ok(())
}
