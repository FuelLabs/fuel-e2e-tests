fuel_e2e_tests::define_fuels!();

use fuel_e2e_tests::setup::{self, Setup};
use fuels::accounts::ViewOnlyAccount;

// Because it has checks for indexation and it broke testnet once because the sdk wasn't
// checking the flags and trying to paginate.
#[tokio::test]
async fn can_call_get_balances() -> color_eyre::Result<()> {
    let Setup { wallet, .. } = setup::init().await?;

    wallet.get_balances().await?;

    Ok(())
}
