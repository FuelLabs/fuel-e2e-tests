use std::str::FromStr;
use fuels::prelude::*;
use fuels::tx::{Bytes32, StorageSlot, Receipt};
use fuels::prelude::Error::TransactionError;
use sha2::{Sha256, Digest};
#[tokio::test]
async fn compile_bindings_from_contract_file() {
    // Generates the bindings from an ABI definition in a JSON file
    // The generated bindings can be accessed through `SimpleContract`.
    abigen!(
        SimpleContract,
        "tests/test_projects/simple_contract/out/debug/simple_contract-abi.json",
    );

    let wallet = launch_provider_and_get_wallet().await;

    // `SimpleContract` is the name of the contract
    let contract_instance = SimpleContract::new(null_contract_id(), wallet);

    let call_handler = contract_instance.takes_ints_returns_bool(42);

    let encoded = format!(
        "{}{}",
        hex::encode(call_handler.contract_call.encoded_selector),
        hex::encode(call_handler.contract_call.encoded_args)
    );

    assert_eq!("000000009593586c000000000000002a", encoded);
}
