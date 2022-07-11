use fuels::prelude::*;

#[tokio::test]
async fn test_tuples() -> Result<(), Error> {
    abigen!(
        MyContract,
        "tests/test_projects/tuples/out/debug/tuples-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/tuples/out/debug/tuples.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = MyContract::new(id.to_string(), wallet.clone());

    let response = instance.returns_tuple((1, 2)).call().await?;

    assert_eq!(response.value, (1, 2));

    // Tuple with struct.
    let my_struct_tuple = (
        42,
        Person {
            name: "Jane".to_string(),
        },
    );
    let response = instance
        .returns_struct_in_tuple(my_struct_tuple.clone())
        .call()
        .await?;

    assert_eq!(response.value, my_struct_tuple);

    // Tuple with enum.
    let my_enum_tuple: (u64, State) = (42, State::A());

    let response = instance
        .returns_enum_in_tuple(my_enum_tuple.clone())
        .call()
        .await?;

    assert_eq!(response.value, my_enum_tuple);

    let id = *ContractId::zeroed();
    let my_b256_u8_tuple: ([u8; 32], u8) = (id, 10);

    let response = instance.tuple_with_b256(my_b256_u8_tuple).call().await?;

    assert_eq!(response.value, my_b256_u8_tuple);
    Ok(())
}
