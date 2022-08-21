use fuel_core::service::Config as CoreConfig;
use fuels::contract::contract::MultiContractCallHandler;
use fuels::prelude::{
    launch_custom_provider_and_get_wallets, launch_provider_and_get_wallet,
    setup_multiple_assets_coins, setup_single_asset_coins, setup_test_provider, Address, AssetId,
    CallParameters, Config, Contract, ContractId, Error, FuelService, LocalWallet, Provider, Salt,
    Signer, StorageConfiguration, TxParameters, WalletsConfig, BASE_ASSET_ID, DEFAULT_COIN_AMOUNT,
    DEFAULT_NUM_COINS,
};
use fuels::tx::Receipt;
use fuels::tx::{Bytes32, StorageSlot};
use std::str::FromStr;
use test_abigen::test_project_abigen;
use test_macros::{test_project_bin_path, test_project_storage_path};

#[tokio::test]
async fn type_safe_output_values() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `SimpleContract`.
    test_project_abigen!(MyContract, "contract_output_test");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("contract_output_test"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();

    // `response`'s type matches the return type of `is_event()`
    let response = contract_instance.is_even(10).call().await?;
    assert!(response.value);

    // `response`'s type matches the return type of `return_my_string()`
    let response = contract_instance
        .return_my_string("fuel".to_string())
        .call()
        .await?;

    assert_eq!(response.value, "fuel");

    let my_struct = MyStruct { foo: 10, bar: true };

    let response = contract_instance.return_my_struct(my_struct).call().await?;

    assert_eq!(response.value.foo, 10);
    assert!(response.value.bar);
    Ok(())
}

#[tokio::test]
async fn call_with_structs() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    // ANCHOR: struct_generation
    test_project_abigen!(MyContract, "complex_types_contract");

    // Here we can use `CounterConfig`, a struct originally
    // defined in the Sway contract.
    let counter_config = CounterConfig {
        dummy: true,
        initial_value: 42,
    };
    // ANCHOR_END: struct_generation

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("complex_types_contract"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();

    let response = contract_instance
        .initialize_counter(counter_config) // Build the ABI call
        .call() // Perform the network call
        .await?;

    assert_eq!(42, response.value);

    let response = contract_instance.increment_counter(10).call().await?;

    assert_eq!(52, response.value);
    Ok(())
}

#[tokio::test]
async fn call_with_empty_return() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    test_project_abigen!(MyContract, "call_empty_return");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("call_empty_return"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();

    let _response = contract_instance
        .store_value(42) // Build the ABI call
        .call() // Perform the network call
        .await?;
    Ok(())
}

#[tokio::test]
async fn test_project_abigen_different_structs_same_arg_name() -> Result<(), Error> {
    test_project_abigen!(MyContract, "two_structs",);

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("two_structs"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();

    let param_one = StructOne { foo: 42 };
    let param_two = StructTwo { bar: 42 };

    let res_one = contract_instance.something(param_one).call().await?;

    assert_eq!(res_one.value, 43);

    let res_two = contract_instance.something_else(param_two).call().await?;

    assert_eq!(res_two.value, 41);
    Ok(())
}

#[tokio::test]
async fn test_reverting_transaction() -> Result<(), Error> {
    test_project_abigen!(RevertingContract, "revert_transaction_error");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("revert_transaction_error"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    let contract_instance = RevertingContractBuilder::new(contract_id.to_string(), wallet).build();
    let response = contract_instance.make_transaction_fail(0).call().await;

    assert!(matches!(response, Err(Error::RevertTransactionError(..))));

    Ok(())
}

#[tokio::test]
async fn multiple_read_calls() -> Result<(), Error> {
    test_project_abigen!(MyContract, "multiple_read_calls");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("multiple_read_calls"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();

    contract_instance.store(42).call().await?;

    // Use "simulate" because the methods don't actually run a transaction, but just a dry-run
    // We can notice here that, thanks to this, we don't generate a TransactionId collision,
    // even if the transactions are theoretically the same.
    let stored = contract_instance.read(0).simulate().await?;

    assert_eq!(stored.value, 42);

    let stored = contract_instance.read(0).simulate().await?;

    assert_eq!(stored.value, 42);
    Ok(())
}

#[tokio::test]
async fn test_methods_typeless_argument() -> Result<(), Error> {
    // Generates the bindings from the an ABI definition inline.
    // The generated bindings can be accessed through `MyContract`.
    test_project_abigen!(MyContract, "empty_arguments");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("empty_arguments"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();

    let response = contract_instance
        .method_with_empty_argument()
        .call()
        .await?;
    assert_eq!(response.value, 63);
    Ok(())
}

#[tokio::test]
async fn test_large_return_data() -> Result<(), Error> {
    test_project_abigen!(MyContract, "large_return_data");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("large_return_data"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();

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
async fn test_contract_calling_contract() -> Result<(), Error> {
    // Tests a contract call that calls another contract (FooCaller calls FooContract underneath)
    test_project_abigen!(FooContract, "foo_contract");

    test_project_abigen!(FooCaller, "foo_caller_contract");

    let wallet = launch_provider_and_get_wallet().await;

    // Load and deploy the first compiled contract
    let foo_contract_id = Contract::deploy(
        test_project_bin_path!("foo_contract"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let foo_contract_instance =
        FooContractBuilder::new(foo_contract_id.to_string(), wallet.clone()).build();

    // Call the contract directly; it just flips the bool value that's passed.
    let res = foo_contract_instance.foo(true).call().await?;
    assert!(!res.value);

    // Load and deploy the second compiled contract
    let foo_caller_contract_id = Contract::deploy(
        test_project_bin_path!("foo_caller_contract"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let foo_caller_contract_instance =
        FooCallerBuilder::new(foo_caller_contract_id.to_string(), wallet.clone()).build();

    // Calls the contract that calls the `FooContract` contract, also just
    // flips the bool value passed to it.
    // ANCHOR: external_contract
    let res = foo_caller_contract_instance
        .call_foo_contract(*foo_contract_id.hash(), true)
        .set_contracts(&[foo_contract_id]) // Sets the external contract
        .call()
        .await?;
    // ANCHOR_END: external_contract

    assert!(!res.value);
    Ok(())
}

#[tokio::test]
async fn test_amount_and_asset_forwarding() -> Result<(), Error> {
    test_project_abigen!(TestFuelCoinContract, "token_ops");

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("token_ops"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = TestFuelCoinContractBuilder::new(id.to_string(), wallet.clone()).build();

    let mut balance_response = instance
        .get_balance((&id).into(), (&id).into())
        .call()
        .await?;
    assert_eq!(balance_response.value, 0);

    instance.mint_coins(5_000_000).call().await?;

    balance_response = instance
        .get_balance(id.clone().into(), (&id).into())
        .call()
        .await?;
    assert_eq!(balance_response.value, 5_000_000);

    let tx_params = TxParameters::new(None, Some(1_000_000), None, None);
    // Forward 1_000_000 coin amount of base asset_id
    // this is a big number for checking that amount can be a u64
    let call_params = CallParameters::new(Some(1_000_000), None, None);

    let response = instance
        .get_msg_amount()
        .tx_params(tx_params)
        .call_params(call_params)
        .call()
        .await?;

    assert_eq!(response.value, 1_000_000);

    let call_response = response
        .receipts
        .iter()
        .find(|&r| matches!(r, Receipt::Call { .. }));

    assert!(call_response.is_some());

    assert_eq!(call_response.unwrap().amount().unwrap(), 1_000_000);
    assert_eq!(call_response.unwrap().asset_id().unwrap(), &BASE_ASSET_ID);

    let address = wallet.address();

    // withdraw some tokens to wallet
    instance
        .transfer_coins_to_output(1_000_000, (&id).into(), address.into())
        .append_variable_outputs(1)
        .call()
        .await?;

    let asset_id = AssetId::from(*id.hash());
    let call_params = CallParameters::new(Some(0), Some(asset_id), None);
    let tx_params = TxParameters::new(None, Some(1_000_000), None, None);

    let response = instance
        .get_msg_amount()
        .tx_params(tx_params)
        .call_params(call_params)
        .call()
        .await?;

    assert_eq!(response.value, 0);

    let call_response = response
        .receipts
        .iter()
        .find(|&r| matches!(r, Receipt::Call { .. }));

    assert!(call_response.is_some());

    assert_eq!(call_response.unwrap().amount().unwrap(), 0);
    assert_eq!(
        call_response.unwrap().asset_id().unwrap(),
        &AssetId::from(*id.hash())
    );
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
async fn test_tuples() -> Result<(), Error> {
    test_project_abigen!(MyContract, "tuples");

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("tuples"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = MyContractBuilder::new(id.to_string(), wallet.clone()).build();

    let response = instance.returns_tuple((1, 2)).call().await?;

    assert_eq!(response.value, (1, 2));

    // Tuple with struct.
    let my_struct_tuple = (
        42,
        Person {
            name: "Jane".to_string(),
        },
    );
    let response = instance
        .returns_struct_in_tuple(my_struct_tuple.clone())
        .call()
        .await?;

    assert_eq!(response.value, my_struct_tuple);

    // Tuple with enum.
    let my_enum_tuple: (u64, State) = (42, State::A());

    let response = instance
        .returns_enum_in_tuple(my_enum_tuple.clone())
        .call()
        .await?;

    assert_eq!(response.value, my_enum_tuple);

    let id = *ContractId::zeroed();
    let my_b256_u8_tuple: ([u8; 32], u8) = (id, 10);

    let response = instance.tuple_with_b256(my_b256_u8_tuple).call().await?;

    assert_eq!(response.value, my_b256_u8_tuple);
    Ok(())
}

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
async fn test_auth_msg_sender_from_sdk() -> Result<(), Error> {
    test_project_abigen!(AuthContract, "auth_testing_contract");

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("auth_testing_contract"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let auth_instance = AuthContractBuilder::new(id.to_string(), wallet.clone()).build();

    // Contract returns true if `msg_sender()` matches `wallet.address()`.
    let response = auth_instance
        .check_msg_sender(wallet.address().into())
        .call()
        .await?;

    assert!(response.value);
    Ok(())
}

#[tokio::test]
async fn workflow_enum_inside_struct() -> Result<(), Error> {
    test_project_abigen!(MyContract, "enum_inside_struct");

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("enum_inside_struct"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    let instance = MyContractBuilder::new(id.to_string(), wallet.clone()).build();
    let response = instance.return_enum_inside_struct(11).call().await?;
    let expected = Cocktail {
        the_thing_you_mix_in: Shaker::Mojito(222),
        glass: 333,
    };
    assert_eq!(response.value, expected);
    let enum_inside_struct = Cocktail {
        the_thing_you_mix_in: Shaker::Cosmopolitan(444),
        glass: 555,
    };
    let response = instance
        .take_enum_inside_struct(enum_inside_struct)
        .call()
        .await?;
    assert_eq!(response.value, 6666);
    Ok(())
}

#[tokio::test]
async fn test_logd_receipts() -> Result<(), Error> {
    test_project_abigen!(LoggingContract, "contract_logdata");

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("contract_logdata"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    let contract_instance = LoggingContractBuilder::new(id.to_string(), wallet.clone()).build();
    let mut value = [0u8; 32];
    value[0] = 0xFF;
    value[1] = 0xEE;
    value[2] = 0xDD;
    value[12] = 0xAA;
    value[13] = 0xBB;
    value[14] = 0xCC;
    let response = contract_instance
        .use_logd_opcode(value, 3, 6)
        .call()
        .await?;
    assert_eq!(response.logs, vec!["ffeedd", "ffeedd000000"]);
    let response = contract_instance
        .use_logd_opcode(value, 14, 15)
        .call()
        .await?;
    assert_eq!(
        response.logs,
        vec![
            "ffeedd000000000000000000aabb",
            "ffeedd000000000000000000aabbcc",
        ]
    );
    let response = contract_instance.dont_use_logd().call().await?;
    assert!(response.logs.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_wallet_balance_api() -> Result<(), Error> {
    // Single asset
    let mut wallet = LocalWallet::new_random(None);
    let number_of_coins = 21;
    let amount_per_coin = 11;
    let coins = setup_single_asset_coins(
        wallet.address(),
        BASE_ASSET_ID,
        number_of_coins,
        amount_per_coin,
    );

    let (provider, _) = setup_test_provider(coins.clone(), None).await;
    wallet.set_provider(provider);
    for (_utxo_id, coin) in coins {
        let balance = wallet.get_asset_balance(&coin.asset_id).await;
        assert_eq!(balance?, number_of_coins * amount_per_coin);
    }
    let balances = wallet.get_balances().await?;
    let expected_key = "0x".to_owned() + BASE_ASSET_ID.to_string().as_str();
    assert_eq!(balances.len(), 1); // only the base asset
    assert!(balances.contains_key(&expected_key));
    assert_eq!(
        *balances.get(&expected_key).unwrap(),
        number_of_coins * amount_per_coin
    );

    // Multiple assets
    let number_of_assets = 7;
    let coins_per_asset = 21;
    let amount_per_coin = 11;
    let (coins, asset_ids) = setup_multiple_assets_coins(
        wallet.address(),
        number_of_assets,
        coins_per_asset,
        amount_per_coin,
    );
    assert_eq!(coins.len() as u64, number_of_assets * coins_per_asset);
    assert_eq!(asset_ids.len() as u64, number_of_assets);
    let (provider, _) = setup_test_provider(coins.clone(), None).await;
    wallet.set_provider(provider);
    let balances = wallet.get_balances().await?;
    assert_eq!(balances.len() as u64, number_of_assets);
    for asset_id in asset_ids {
        let balance = wallet.get_asset_balance(&asset_id).await;
        assert_eq!(balance?, coins_per_asset * amount_per_coin);
        let expected_key = "0x".to_owned() + asset_id.to_string().as_str();
        assert!(balances.contains_key(&expected_key));
        assert_eq!(
            *balances.get(&expected_key).unwrap(),
            coins_per_asset * amount_per_coin
        );
    }
    Ok(())
}

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

#[tokio::test]
async fn test_transaction_script_workflow() -> Result<(), Error> {
    test_project_abigen!(MyContract, "contract_test");

    let wallet = launch_provider_and_get_wallet().await;
    let provider = &wallet.get_provider()?;

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

    let receipts = script.call(provider).await?;

    let response = call_handler.get_response(receipts)?;
    assert_eq!(response.value, 42);
    Ok(())
}

#[tokio::test]
async fn enum_coding_w_variable_width_variants() -> Result<(), Error> {
    test_project_abigen!(EnumTesting, "enum_encoding");

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("enum_encoding"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = EnumTestingBuilder::new(id.to_string(), wallet).build();

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

#[tokio::test]
async fn enum_coding_w_unit_enums() -> Result<(), Error> {
    test_project_abigen!(EnumTesting, "enum_encoding");

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("enum_encoding"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = EnumTestingBuilder::new(id.to_string(), wallet).build();

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

#[tokio::test]
async fn nested_structs() -> Result<(), Error> {
    test_project_abigen!(NestedStructs, "nested_structs");

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("nested_structs"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = NestedStructsBuilder::new(id.to_string(), wallet).build();

    let expected = AllStruct {
        some_struct: SomeStruct { par_1: 12345 },
    };

    let actual = instance.get_struct().call().await?.value;
    assert_eq!(actual, expected);

    let fuelvm_judgement = instance
        .check_struct_integrity(expected)
        .call()
        .await?
        .value;

    assert!(
        fuelvm_judgement,
        "The FuelVM deems that we've not encoded the argument correctly. Investigate!"
    );

    let memory_address = MemoryAddress {
        contract_id: ContractId::zeroed(),
        function_selector: 10,
        function_data: 0,
    };

    let call_data = CallData {
        memory_address,
        num_coins_to_forward: 10,
        asset_id_of_coins_to_forward: ContractId::zeroed(),
        amount_of_gas_to_forward: 5,
    };

    let actual = instance
        .nested_struct_with_reserved_keyword_substring(call_data.clone())
        .call()
        .await?
        .value;

    assert_eq!(actual, call_data);
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
    let provider = &wallet.get_provider()?;

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
    let receipts = script.call(provider).await.unwrap();
    let (counter, array) = multi_call_handler
        .get_response::<(u64, Vec<u64>)>(receipts)?
        .value;

    assert_eq!(counter, 42);
    assert_eq!(array, [42; 2]);

    Ok(())
}

#[tokio::test]
async fn test_storage_initialization() -> Result<(), Error> {
    test_project_abigen!(MyContract, "contract_storage_test");

    let wallet = launch_provider_and_get_wallet().await;

    // ANCHOR: storage_slot_create
    let key = Bytes32::from([1u8; 32]);
    let value = Bytes32::from([2u8; 32]);
    let storage_slot = StorageSlot::new(key, value);
    let storage_vec = vec![storage_slot.clone()];
    // ANCHOR_END: storage_slot_create

    // ANCHOR: manual_storage
    let contract_id = Contract::deploy_with_parameters(
        test_project_bin_path!("contract_storage_test"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_manual_storage(Some(storage_vec)),
        Salt::from([0; 32]),
    )
    .await?;
    // ANCHOR_END: manual_storage

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();

    let result = contract_instance
        .get_value_b256(key.into())
        .call()
        .await?
        .value;
    assert_eq!(result.as_slice(), value.as_slice());

    Ok(())
}

#[tokio::test]
async fn can_use_try_into_to_construct_struct_from_bytes() -> Result<(), Error> {
    test_project_abigen!(MyContract, "enum_inside_struct");
    let cocktail_in_bytes: Vec<u8> = vec![
        0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3,
    ];

    let expected = Cocktail {
        the_thing_you_mix_in: Shaker::Mojito(2),
        glass: 3,
    };

    // as slice
    let actual: Cocktail = cocktail_in_bytes[..].try_into()?;
    assert_eq!(actual, expected);

    // as ref
    let actual: Cocktail = (&cocktail_in_bytes).try_into()?;
    assert_eq!(actual, expected);

    // as value
    let actual: Cocktail = cocktail_in_bytes.try_into()?;
    assert_eq!(actual, expected);

    Ok(())
}

#[tokio::test]
async fn can_use_try_into_to_construct_enum_from_bytes() -> Result<(), Error> {
    test_project_abigen!(MyContract, "enum_inside_struct");
    // ANCHOR: manual_decode
    let shaker_in_bytes: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2];

    let expected = Shaker::Mojito(2);

    // as slice
    let actual: Shaker = shaker_in_bytes[..].try_into()?;
    assert_eq!(actual, expected);

    // as ref
    let actual: Shaker = (&shaker_in_bytes).try_into()?;
    assert_eq!(actual, expected);

    // as value
    let actual: Shaker = shaker_in_bytes.try_into()?;
    assert_eq!(actual, expected);

    // ANCHOR_END: manual_decode

    Ok(())
}

#[tokio::test]
async fn type_inside_enum() -> Result<(), Error> {
    test_project_abigen!(MyContract, "type_inside_enum");

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("type_inside_enum"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = MyContractBuilder::new(id.to_string(), wallet.clone()).build();

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

#[tokio::test]
async fn test_init_storage_automatically() -> Result<(), Error> {
    test_project_abigen!(MyContract, "contract_storage_test");

    let wallet = launch_provider_and_get_wallet().await;

    // ANCHOR: automatic_storage
    let contract_id = Contract::deploy_with_parameters(
        test_project_bin_path!("contract_storage_test"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_storage_path(Some(
            test_project_storage_path!("contract_storage_test").to_string(),
        )),
        Salt::default(),
    )
    .await?;
    // ANCHOR_END: automatic_storage

    let key1 =
        Bytes32::from_str("de9090cb50e71c2588c773487d1da7066d0c719849a7e58dc8b6397a25c567c0")
            .unwrap();
    let key2 =
        Bytes32::from_str("f383b0ce51358be57daa3b725fe44acdb2d880604e367199080b4379c41bb6ed")
            .unwrap();

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet.clone()).build();

    let value = contract_instance.get_value_b256(*key1).call().await?.value;
    assert_eq!(value, [1u8; 32]);

    let value = contract_instance.get_value_u64(*key2).call().await?.value;
    assert_eq!(value, 64);

    Ok(())
}

#[tokio::test]
async fn test_init_storage_automatically_bad_json_path() -> Result<(), Error> {
    test_project_abigen!(MyContract, "contract_storage_test");

    let wallet = launch_provider_and_get_wallet().await;

    let response = Contract::deploy_with_parameters(
        test_project_bin_path!("contract_storage_test"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_storage_path(
            Some("tests/test_projects/contract_storage_test/out/debug/contract_storage_test-storage_slts.json".to_string())),
        Salt::default(),
    ).await.expect_err("Should fail");

    let expected = "Invalid data:";
    assert!(response.to_string().starts_with(expected));

    Ok(())
}

#[tokio::test]
async fn contract_method_call_respects_maturity() -> Result<(), Error> {
    test_project_abigen!(MyContract, "transaction_block_height");

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("transaction_block_height"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = MyContractBuilder::new(id.to_string(), wallet.clone()).build();

    let call_w_maturity = |call_maturity| {
        let mut prepared_call = instance.calling_this_will_produce_a_block();
        prepared_call.tx_parameters.maturity = call_maturity;
        prepared_call.call()
    };

    call_w_maturity(1).await.expect("Should have passed since we're calling with a maturity that is less or equal to the current block height");

    call_w_maturity(3).await.expect_err("Should have failed since we're calling with a maturity that is greater than the current block height");

    Ok(())
}

#[tokio::test]
async fn contract_deployment_respects_maturity() -> Result<(), Error> {
    test_project_abigen!(MyContract, "transaction_block_height");

    let config = Config {
        manual_blocks_enabled: true,
        ..Config::local_node()
    };
    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), Some(config)).await;
    let wallet = &wallets[0];
    let provider = wallet.get_provider()?;

    let deploy_w_maturity = |maturity| {
        let parameters = TxParameters {
            maturity,
            ..TxParameters::default()
        };
        Contract::deploy(
            test_project_bin_path!("transaction_block_height"),
            wallet,
            parameters,
            StorageConfiguration::default(),
        )
    };

    let err = deploy_w_maturity(1).await.expect_err("Should not have been able to deploy the contract since the block height (0) is less than the requested maturity (1)");
    assert!(matches!(
        err,
        Error::ValidationError(fuels::tx::ValidationError::TransactionMaturity)
    ));

    provider.produce_blocks(1).await?;
    deploy_w_maturity(1)
        .await
        .expect("Should be able to deploy now since maturity (1) is <= than the block height (1)");

    Ok(())
}

#[tokio::test]
async fn can_increase_block_height() -> Result<(), Error> {
    // ANCHOR: use_produce_blocks_to_increase_block_height
    let config = Config {
        manual_blocks_enabled: true, // Necessary so the `produce_blocks` API can be used locally
        ..Config::local_node()
    };
    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::default(), Some(config)).await;
    let wallet = &wallets[0];
    let provider = wallet.get_provider()?;

    assert_eq!(provider.latest_block_height().await?, 0);

    provider.produce_blocks(3).await?;

    assert_eq!(provider.latest_block_height().await?, 3);
    // ANCHOR_END: use_produce_blocks_to_increase_block_height
    Ok(())
}

#[tokio::test]
async fn can_handle_sway_function_called_new() -> anyhow::Result<()> {
    test_project_abigen!(MyContract, "collision_in_fn_names");

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("collision_in_fn_names"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let instance = MyContractBuilder::new(id.to_string(), wallet.clone()).build();

    let response = instance.new().call().await?.value;

    assert_eq!(response, 12345);

    Ok(())
}

#[tokio::test]
async fn test_get_gas_used() -> anyhow::Result<()> {
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

    let gas_used = instance.initialize_counter(42).call().await?.gas_used;

    assert!(gas_used > 0);

    Ok(())
}

#[tokio::test]
async fn test_network_error() -> Result<(), anyhow::Error> {
    test_project_abigen!(MyContract, "contract_test");

    let mut wallet = LocalWallet::new_random(None);

    let config = CoreConfig::local_node();
    let service = FuelService::new_node(config).await?;
    let provider = Provider::connect(service.bound_address).await?;

    wallet.set_provider(provider);

    // Simulate an unreachable node
    service.stop().await;

    let response = Contract::deploy(
        test_project_bin_path!("contract_test"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await;

    assert!(matches!(response, Err(Error::ProviderError(_))));

    Ok(())
}

#[tokio::test]
async fn str_in_array() -> Result<(), Error> {
    test_project_abigen!(MyContract, "str_in_array");

    let wallet = launch_provider_and_get_wallet().await;

    let contract_id = Contract::deploy(
        test_project_bin_path!("str_in_array"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let contract_instance = MyContractBuilder::new(contract_id.to_string(), wallet).build();

    let input = vec!["foo".to_string(), "bar".to_string(), "baz".to_string()];
    let response = contract_instance
        .take_array_string_shuffle(input.clone())
        .call()
        .await?;

    assert_eq!(response.value, ["baz", "foo", "bar"]);

    let response = contract_instance
        .take_array_string_return_single(input.clone())
        .call()
        .await?;

    assert_eq!(response.value, ["foo"]);

    let response = contract_instance
        .take_array_string_return_single_element(input)
        .call()
        .await?;

    assert_eq!(response.value, "bar");

    Ok(())
}

#[tokio::test]
async fn test_connect_wallet() -> anyhow::Result<()> {
    test_project_abigen!(MyContract, "contract_test");

    let config = WalletsConfig::new(Some(2), Some(1), Some(DEFAULT_COIN_AMOUNT));

    let mut wallets = launch_custom_provider_and_get_wallets(config, None).await;
    let wallet_1 = wallets.pop().unwrap();
    let wallet_2 = wallets.pop().unwrap();

    let id = Contract::deploy(
        test_project_bin_path!("contract_test"),
        &wallet_1,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    // pay for call with wallet_1
    let contract_instance = MyContractBuilder::new(id.to_string(), wallet_1.clone()).build();
    let tx_params = TxParameters::new(Some(10), Some(10000), None, None);
    contract_instance
        .initialize_counter(42)
        .tx_params(tx_params)
        .call()
        .await?;

    // confirm that funds have been deducted
    let wallet_1_balance = wallet_1.get_asset_balance(&Default::default()).await?;
    assert!(DEFAULT_COIN_AMOUNT > wallet_1_balance);

    // pay for call with wallet_2
    contract_instance
        ._with_wallet(wallet_2.clone())?
        .initialize_counter(42)
        .tx_params(tx_params)
        .call()
        .await?;

    // confirm there are no changes to wallet_1, wallet_2 has been charged
    let wallet_1_balance_second_call = wallet_1.get_asset_balance(&Default::default()).await?;
    let wallet_2_balance = wallet_2.get_asset_balance(&Default::default()).await?;
    assert_eq!(wallet_1_balance_second_call, wallet_1_balance);
    assert!(DEFAULT_COIN_AMOUNT > wallet_2_balance);

    Ok(())
}
