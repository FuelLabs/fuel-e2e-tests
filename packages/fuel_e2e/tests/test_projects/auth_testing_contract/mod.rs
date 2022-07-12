use fuels::prelude::*;
use third::test_project_bin_path;

#[tokio::test]
async fn test_auth_msg_sender_from_sdk() -> Result<(), Error> {
    abigen!(
        AuthContract,
        "packages/fuel_e2e/tests/test_projects/auth_testing_contract/out/debug/auth_testing_contract-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        test_project_bin_path!("auth_testing_contract"),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;

    let auth_instance = AuthContract::new(id.to_string(), wallet.clone());

    // Contract returns true if `msg_sender()` matches `wallet.address()`.
    let response = auth_instance
        .check_msg_sender(wallet.address())
        .call()
        .await?;

    assert!(response.value);
    Ok(())
}
