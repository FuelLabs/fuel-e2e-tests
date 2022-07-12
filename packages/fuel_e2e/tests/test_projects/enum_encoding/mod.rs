use fuels::prelude::*;
use third::test_project_bin_path;

#[tokio::test]
async fn enum_coding_w_unit_enums() -> Result<(), Error> {
    abigen!(
        EnumTesting,
        "packages/fuel_e2e/tests/test_projects/enum_encoding/out/debug\
        /enum_encoding-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("enum_encoding"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = EnumTesting::new(id.to_string(), wallet);

    // If we had a regression on the issue of unit enum encoding width, then
    // we'll end up mangling arg_2
    let expected = UnitBundle {
        arg_1: UnitEnum::var2(),
        arg_2: u64::MAX,
    };
    let actual = instance.get_unit_bundle().call().await?.value;
    assert_eq!(actual, expected);

    let fuelvm_judgement = instance
        .check_unit_bundle_integrity(expected)
        .call()
        .await?
        .value;

    assert!(
        fuelvm_judgement,
        "The FuelVM deems that we've not encoded the bundle correctly. Investigate!"
    );
    Ok(())
}
#[tokio::test]
async fn enum_coding_w_variable_width_variants() -> Result<(), Error> {
    abigen!(
        EnumTesting,
        "packages/fuel_e2e/tests/test_projects/enum_encoding/out/debug\
        /enum_encoding-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("enum_encoding"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = EnumTesting::new(id.to_string(), wallet);

    // If we had a regression on the issue of enum encoding width, then we'll
    // probably end up mangling arg_2 and onward which will fail this test.
    let expected = BigBundle {
        arg_1: EnumThatHasABigAndSmallVariant::Small(12345),
        arg_2: 6666,
        arg_3: 7777,
        arg_4: 8888,
    };
    let actual = instance.get_big_bundle().call().await?.value;
    assert_eq!(actual, expected);

    let fuelvm_judgement = instance
        .check_big_bundle_integrity(expected)
        .call()
        .await?
        .value;

    assert!(
        fuelvm_judgement,
        "The FuelVM deems that we've not encoded the bundle correctly. Investigate!"
    );
    Ok(())
}
