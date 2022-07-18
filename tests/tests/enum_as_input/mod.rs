use fuels::prelude::*;
use test_abigen::test_project_abigen;
use test_macros::test_project_bin_path;

#[tokio::test]
async fn enum_as_input() -> Result<(), Error> {
    test_project_abigen!(EnumTesting, "enum_as_input");

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("enum_as_input"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = EnumTestingBuilder::new(id.to_string(), wallet).build();

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
