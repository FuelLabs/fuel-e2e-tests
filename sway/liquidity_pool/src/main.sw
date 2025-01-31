contract;

use std::{
    asset::{
        mint_to,
        transfer,
    },
    call_frames::{
        msg_asset_id,
    },
    constants::ZERO_B256,
    context::msg_amount,
};

abi LiquidityPool {
    #[storage(read, write)]
    #[payable]
    fn deposit(recipient: Identity);
    #[payable]
    fn withdraw(recipient: Identity);
    #[storage(read)]
    fn total_deposited_ever() -> u64;
}

storage {
    total_deposited_ever: u64 = 0,
}

impl LiquidityPool for Contract {
    #[storage(read, write)]
    #[payable]
    fn deposit(recipient: Identity) {
        assert(AssetId::base() == msg_asset_id());
        assert(0 < msg_amount());

        // Mint two times the amount.
        let amount_to_mint = msg_amount() * 2;

        // Mint some LP token based upon the amount of the base token.
        mint_to(recipient, ZERO_B256, amount_to_mint);

        storage
            .total_deposited_ever
            .write(storage.total_deposited_ever.read() + msg_amount());
    }

    #[payable]
    fn withdraw(recipient: Identity) {
        assert(0 < msg_amount());

        // Amount to withdraw.
        let amount_to_transfer = msg_amount() / 2;

        // Transfer base token to recipient.
        transfer(recipient, AssetId::base(), amount_to_transfer);
    }

    #[storage(read)]
    fn total_deposited_ever() -> u64 {
        storage.total_deposited_ever.read()
    }
}
