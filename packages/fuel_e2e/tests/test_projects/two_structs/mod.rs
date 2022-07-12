use fuels::prelude::*;
use some_macros::test_project_abigen;
use third::test_project_bin_path;

#[tokio::test]
async fn abigen_different_structs_same_arg_name() -> Result<(), Error> {
    test_project_abigen!(MyContract, "two_structs",);

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("two_structs"),
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
