use fuels::prelude::*;

#[tokio::test]
async fn abigen_different_structs_same_arg_name() -> Result<(), Error> {
    abigen!(
        MyContract,
        "tests/test_projects/two_structs/out/debug/two_structs-abi.json",
    );

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        "tests/test_projects/two_structs/out/debug/two_structs.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance = MyContract::new(contract_id.to_string(), wallet);

    let param_one = StructOne { foo: 42 };
    let param_two = StructTwo { bar: 42 };

    let res_one = contract_instance.something(param_one).call().await?;

    assert_eq!(res_one.value, 43);

    let res_two = contract_instance.something_else(param_two).call().await?;

    assert_eq!(res_two.value, 41);
    Ok(())
}
