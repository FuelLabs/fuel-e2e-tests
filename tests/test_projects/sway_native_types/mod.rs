use fuels::prelude::*;
use std::str::FromStr;

#[tokio::test]
async fn sway_native_types_support() -> Result<(), Box<dyn std::error::Error>> {
    abigen!(
        MyContract,
        "tests/test_projects/sway_native_types/out/debug/sway_native_types-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/sway_native_types/out/debug/sway_native_types.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = MyContract::new(id.to_string(), wallet.clone());

    let user = User {
        weight: 10,
        address: Address::zeroed(),
    };
    let response = instance.wrapped_address(user).call().await?;

    assert_eq!(response.value.address, Address::zeroed());

    let response = instance.unwrapped_address(Address::zeroed()).call().await?;

    assert_eq!(
        response.value,
        Address::from_str("0x0000000000000000000000000000000000000000000000000000000000000000")?
    );
    Ok(())
}
