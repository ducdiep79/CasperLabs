#![no_std]

use contract_ffi::{
    block_time::BlockTime,
    contract_api::{runtime, Error},
    unwrap_or_revert::UnwrapOrRevert,
};

#[no_mangle]
pub extern "C" fn call() {
    let known_block_time: u64 = runtime::get_arg(0)
        .unwrap_or_revert_with(Error::MissingArgument)
        .unwrap_or_revert_with(Error::InvalidArgument);
    let actual_block_time: BlockTime = runtime::get_blocktime();

    assert_eq!(
        actual_block_time,
        BlockTime::new(known_block_time),
        "actual block time not known block time"
    );
}
