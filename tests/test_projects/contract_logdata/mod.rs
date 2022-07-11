use fuels::prelude::*;

#[tokio::test]
async fn test_logd_receipts() -> Result<(), Error> {
    abigen!(
        LoggingContract,
        "tests/test_projects/contract_logdata/out/debug/contract_logdata-abi.json"
    );

    let wallet = launch_provider_and_get_wallet().await;

    let id = Contract::deploy(
        "tests/test_projects/contract_logdata/out/debug/contract_logdata.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::default(),
    )
    .await?;
    let contract_instance = LoggingContract::new(id.to_string(), wallet.clone());
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
