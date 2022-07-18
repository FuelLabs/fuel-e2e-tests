use fuels::prelude::*;
use test_abigen::test_project_abigen;
use test_macros::test_project_bin_path;

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
