use std::collections::HashMap;

use crate::support::test_support::{
    InMemoryWasmTestBuilder, DEFAULT_BLOCK_TIME, STANDARD_PAYMENT_CONTRACT,
};
use contract_ffi::value::U512;
use engine_core::engine_state::MAX_PAYMENT;

const GENESIS_ADDR: [u8; 32] = [7u8; 32];
const ACCOUNT_1_ADDR: [u8; 32] = [1u8; 32];

const ACCOUNT_1_INITIAL_BALANCE: u64 = MAX_PAYMENT * 2;

#[ignore]
#[test]
fn should_have_read_only_access_to_system_contract_urefs() {
    let mut builder = InMemoryWasmTestBuilder::default();

    builder
        .run_genesis(GENESIS_ADDR, HashMap::new())
        .exec_with_args(
            GENESIS_ADDR,
            STANDARD_PAYMENT_CONTRACT,
            (U512::from(MAX_PAYMENT),),
            "transfer_purse_to_account.wasm",
            (ACCOUNT_1_ADDR, U512::from(ACCOUNT_1_INITIAL_BALANCE)),
            DEFAULT_BLOCK_TIME,
            [1u8; 32],
        )
        .commit()
        .exec(
            ACCOUNT_1_ADDR,
            "check_system_contract_urefs_access_rights.wasm",
            DEFAULT_BLOCK_TIME,
            [2u8; 32],
        )
        .commit()
        .expect_success();
}
