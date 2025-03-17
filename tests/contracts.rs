define_fuels!();

use fuel_e2e_tests::{
    define_fuels, helpers,
    setup::{self, Setup},
};
use fuels::{macros::abigen, programs::calls::CallHandler};

#[tokio::test]
async fn multi_call() -> color_eyre::Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "sway/contract_test/out/release/contract_test-abi.json"
    ));

    let Setup {
        wallet,
        deploy_config,
    } = setup::init().await?;

    let contract_id = helpers::deploy(
        &wallet,
        deploy_config,
        "sway/contract_test/out/release/contract_test.bin",
    )
    .await?;

    let contract_methods = MyContract::new(contract_id, wallet.clone()).methods();

    let multi_call_handler = CallHandler::new_multi_call(wallet.clone())
        .add_call(contract_methods.initialize_counter(0))
        .add_call(contract_methods.increment_counter(3))
        .add_call(contract_methods.increment_counter(6))
        .add_call(contract_methods.increment_counter(9));

    let response_value: (u64, u64, u64, u64) = multi_call_handler.call().await?.value;

    assert_eq!(response_value, (0, 3, 9, 18));

    Ok(())
}
