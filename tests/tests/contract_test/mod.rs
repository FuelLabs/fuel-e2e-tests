use fuels::prelude::*;
use test_abigen::test_project_abigen;
use test_macros::test_project_bin_path;

#[tokio::test]
async fn test_array() -> Result<(), Error> {
    test_project_abigen!(MyContract, "contract_test");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("contract_test"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    println!("Contract deployed @ {:x}", contract_id);
    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();

    assert_eq!(
        contract_instance
            .get_array([42; 2].to_vec())
            .call()
            .await?
            .value,
        [42; 2]
    );
    Ok(())
}
#[tokio::test]
async fn test_arrays_with_custom_types() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    test_project_abigen!(MyContract, "contract_test");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("contract_test"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    println!("Contract deployed @ {:x}", contract_id);
    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();

    let persons = vec![
        Person {
            name: "John".to_string(),
        },
        Person {
            name: "Jane".to_string(),
        },
    ];

    let response = contract_instance.array_of_structs(persons).call().await?;

    assert_eq!("John", response.value[0].name);
    assert_eq!("Jane", response.value[1].name);

    let states = vec![State::A(), State::B()];

    let response = contract_instance
        .array_of_enums(states.clone())
        .call()
        .await?;

    assert_eq!(states[0], response.value[0]);
    assert_eq!(states[1], response.value[1]);
    Ok(())
}
#[tokio::test]
async fn test_call_param_gas_errors() -> Result<(), Error> {
    test_project_abigen!(MyContract, "contract_test");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("contract_test"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();

    // Transaction gas_limit is sufficient, call gas_forwarded is too small
    let response = contract_instance
        .initialize_counter(42)
        .tx_params(TxParameters::new(None, Some(1000), None, None))
        .call_params(CallParameters::new(None, None, Some(1)))
        .call()
        .await
        .expect_err("should error");

    let expected = "Contract call error: OutOfGas, receipts:";
    assert!(response.to_string().starts_with(expected));

    // Call params gas_forwarded exceeds transaction limit
    let response = contract_instance
        .initialize_counter(42)
        .tx_params(TxParameters::new(None, Some(1), None, None))
        .call_params(CallParameters::new(None, None, Some(1000)))
        .call()
        .await
        .expect_err("should error");

    let expected = "Contract call error: OutOfGas, receipts:";
    assert!(response.to_string().starts_with(expected));
    Ok(())
}
#[tokio::test]
async fn test_gas_errors() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    test_project_abigen!(MyContract, "contract_test");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("contract_test"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();

    // Test for insufficient gas.
    let response = contract_instance
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(
            Some(DEFAULT_COIN_AMOUNT),
            Some(100),
            None,
            None,
        ))
        .call() // Perform the network call
        .await
        .expect_err("should error");

    let expected = "Contract call error: OutOfGas, receipts:";
    assert!(response.to_string().starts_with(expected));

    // Test for running out of gas. Gas price as `None` will be 0.
    // Gas limit will be 100, this call will use more than 100 gas.
    let response = contract_instance
        .initialize_counter(42) // Build the ABI call
        .tx_params(TxParameters::new(None, Some(100), None, None))
        .call() // Perform the network call
        .await
        .expect_err("should error");

    let expected = "Contract call error: OutOfGas, receipts:";

    assert!(response.to_string().starts_with(expected));
    Ok(())
}
#[tokio::test]
async fn test_multi_call() -> Result<(), Error> {
    test_project_abigen!(MyContract, "contract_test");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("contract_test"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();

    let call_handler_1 = contract_instance.initialize_counter(42);
    let call_handler_2 = contract_instance.get_array([42; 2].to_vec());

    let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

    multi_call_handler
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let (counter, array): (u64, Vec<u64>) = multi_call_handler.call().await?.value;

    assert_eq!(counter, 42);
    assert_eq!(array, [42; 2]);

    Ok(())
}
#[tokio::test]
async fn test_multi_call_script_workflow() -> Result<(), Error> {
    test_project_abigen!(MyContract, "contract_test");

    let wallet = launch_provider_and_get_wallet().await;
    let client = &wallet.get_provider()?.client;

    let contract_id = Contract::deploy(
        test_project_bin_path!("contract_test"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();

    let call_handler_1 = contract_instance.initialize_counter(42);
    let call_handler_2 = contract_instance.get_array([42; 2].to_vec());

    let mut multi_call_handler = MultiContractCallHandler::new(wallet.clone());

    multi_call_handler
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let script = multi_call_handler.get_script().await;
    let receipts = script.call(client).await.unwrap();
    let (counter, array) = multi_call_handler
        .get_response::<(u64, Vec<u64>)>(receipts)?
        .value;

    assert_eq!(counter, 42);
    assert_eq!(array, [42; 2]);

    Ok(())
}
#[tokio::test]
async fn test_multiple_args() -> Result<(), Error> {
    test_project_abigen!(MyContract, "contract_test");

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("contract_test"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = MyContractBuilder::new(id.to_string(), wallet.clone()).build();

    // Make sure we can call the contract with multiple arguments
    let response = instance.get(5, 6).call().await?;

    assert_eq!(response.value, 5);

    let t = MyType { x: 5, y: 6 };
    let response = instance.get_alt(t.clone()).call().await?;
    assert_eq!(response.value, t);

    let response = instance.get_single(5).call().await?;
    assert_eq!(response.value, 5);
    Ok(())
}
#[tokio::test]
async fn test_provider_launch_and_connect() -> Result<(), Error> {
    test_project_abigen!(MyContract, "contract_test");

    let mut wallet = LocalWallet::new_random(None);

    let coins = setup_single_asset_coins(
        wallet.address(),
        BASE_ASSET_ID,
        DEFAULT_NUM_COINS,
        DEFAULT_COIN_AMOUNT,
    );
    let (launched_provider, address) = setup_test_provider(coins, None).await;
    let connected_provider = Provider::connect(address).await?;

    wallet.set_provider(connected_provider);

    let contract_id = Contract::deploy(
        test_project_bin_path!("contract_test"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    println!("Contract deployed @ {:x}", contract_id);

    let contract_instance_connected =
        MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();

    let response = contract_instance_connected
        .initialize_counter(42) // Build the ABI call
        .call() // Perform the network call
        .await?;
    assert_eq!(42, response.value);

    wallet.set_provider(launched_provider);
    let contract_instance_launched =
        MyContractBuilder::new(contract_id.to_string(), wallet).build();

    let response = contract_instance_launched
        .increment_counter(10)
        .call()
        .await?;
    assert_eq!(52, response.value);
    Ok(())
}
#[tokio::test]
async fn test_transaction_script_workflow() -> Result<(), Error> {
    test_project_abigen!(MyContract, "contract_test");

    let wallet = launch_provider_and_get_wallet().await;
    let client = &wallet.get_provider()?.client;

    let contract_id = Contract::deploy(
        test_project_bin_path!("contract_test"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();

    let call_handler = contract_instance.initialize_counter(42);

    let script = call_handler.get_script().await;
    assert!(script.tx.is_script());

    let receipts = script.call(client).await?;

    let response = call_handler.get_response(receipts)?;
    assert_eq!(response.value, 42);
    Ok(())
}
