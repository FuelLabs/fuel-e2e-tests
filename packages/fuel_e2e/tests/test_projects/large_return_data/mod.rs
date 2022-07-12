use fuels::prelude::*;
use third::test_project_bin_path;

#[tokio::test]
async fn test_large_return_data() -> Result<(), Error> {
    abigen!(
        MyContract,
        "packages/fuel_e2e/tests/test_projects/large_return_data/out/debug/large_return_data-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("large_return_data"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

    let res = contract_instance.get_id().call().await?;

    assert_eq!(
        res.value,
        [
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
        ]
    );

    // One word-sized string
    let res = contract_instance.get_small_string().call().await?;
    assert_eq!(res.value, "gggggggg");

    // Two word-sized string
    let res = contract_instance.get_large_string().call().await?;
    assert_eq!(res.value, "ggggggggg");

    // Large struct will be bigger than a `WORD`.
    let res = contract_instance.get_large_struct().call().await?;
    assert_eq!(res.value.foo, 12);
    assert_eq!(res.value.bar, 42);

    // Array will be returned in `ReturnData`.
    let res = contract_instance.get_large_array().call().await?;
    assert_eq!(res.value, &[1, 2]);

    let res = contract_instance.get_contract_id().call().await?;

    // First `value` is from `CallResponse`.
    // Second `value` is from Sway `ContractId` type.
    assert_eq!(
        res.value,
        ContractId::from([
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
        ])
    );
    Ok(())
}
