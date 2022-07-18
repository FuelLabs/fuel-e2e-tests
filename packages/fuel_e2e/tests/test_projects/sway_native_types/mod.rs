use fuels::prelude::*;
use std::str::FromStr;
use test_abigen::test_project_abigen;
use test_macros::test_project_bin_path;

#[tokio::test]
async fn sway_native_types_support() -> Result<(), Box<dyn std::error::Error>> {
    test_project_abigen!(MyContract, "sway_native_types");

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("sway_native_types"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = MyContractBuilder::new(id.to_string(), wallet.clone()).build();

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
