use fuels::prelude::*;

#[tokio::test]
async fn type_inside_enum() -> Result<(), Error> {
    abigen!(
        MyContract,
        "tests/test_projects/type_inside_enum/out/debug\
        /type_inside_enum-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/type_inside_enum/out/debug/type_inside_enum.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = MyContract::new(id.to_string(), wallet.clone());

    // String inside enum
    let enum_string = SomeEnum::SomeStr("asdf".to_owned());
    let response = instance.str_inside_enum(enum_string.clone()).call().await?;
    assert_eq!(response.value, enum_string);

    // Array inside enum
    let enum_array = SomeEnum::SomeArr(vec![1, 2, 3, 4, 5, 6, 7]);
    let response = instance.arr_inside_enum(enum_array.clone()).call().await?;
    assert_eq!(response.value, enum_array);

    // Struct inside enum
    let response = instance.return_struct_inside_enum(11).call().await?;
    let expected = Shaker::Cosmopolitan(Recipe { ice: 22, sugar: 99 });
    assert_eq!(response.value, expected);
    let struct_inside_enum = Shaker::Cosmopolitan(Recipe { ice: 22, sugar: 66 });
    let response = instance
        .take_struct_inside_enum(struct_inside_enum)
        .call()
        .await?;
    assert_eq!(response.value, 8888);

    // Enum inside enum
    let expected_enum = EnumLevel3::El2(EnumLevel2::El1(EnumLevel1::Num(42)));
    let response = instance.get_nested_enum().call().await?;
    assert_eq!(response.value, expected_enum);

    let response = instance
        .check_nested_enum_integrity(expected_enum)
        .call()
        .await?;
    assert!(
        response.value,
        "The FuelVM deems that we've not encoded the nested enum correctly. Investigate!"
    );

    Ok(())
}
