use fuels::prelude::*;

#[tokio::test]
async fn enum_as_input() -> Result<(), Error> {
    abigen!(
        EnumTesting,
        "tests/test_projects/enum_as_input/out/debug\
        /enum_as_input-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/enum_as_input/out/debug/enum_as_input.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = EnumTesting::new(id.to_string(), wallet);

    let expected = StandardEnum::Two(12345);
    let actual = instance.get_standard_enum().call().await?.value;
    assert_eq!(expected, actual);

    let fuelvm_judgement = instance
        .check_standard_enum_integrity(expected)
        .call()
        .await?
        .value;
    assert!(
        fuelvm_judgement,
        "The FuelVM deems that we've not encoded the standard enum correctly. Investigate!"
    );

    let expected = UnitEnum::Two();
    let actual = instance.get_unit_enum().call().await?.value;
    assert_eq!(actual, expected);

    let fuelvm_judgement = instance
        .check_unit_enum_integrity(expected)
        .call()
        .await?
        .value;
    assert!(
        fuelvm_judgement,
        "The FuelVM deems that we've not encoded the unit enum correctly. Investigate!"
    );
    Ok(())
}
