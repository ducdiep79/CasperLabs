use crate::support::test_support::{ExecuteRequestBuilder, InMemoryWasmTestBuilder};
use contract_ffi::key::Key;
use contract_ffi::value::Value;

use crate::test::{DEFAULT_ACCOUNT_ADDR, DEFAULT_GENESIS_CONFIG, DEFAULT_PAYMENT};

const CONTRACT_MAIN_PURSE: &str = "main_purse";
const CONTRACT_TRANSFER_PURSE_TO_ACCOUNT: &str = "transfer_purse_to_account";
const ACCOUNT_1_ADDR: [u8; 32] = [1u8; 32];

#[ignore]
#[test]
fn should_run_main_purse_contract_default_account() {
    let mut builder = InMemoryWasmTestBuilder::default();

    let builder = builder.run_genesis(&DEFAULT_GENESIS_CONFIG);

    let default_account = if let Some(Value::Account(account)) =
        builder.query(None, Key::Account(DEFAULT_ACCOUNT_ADDR), &[])
    {
        account
    } else {
        panic!("could not get account")
    };

    let exec_request = {
        let contract_name = format!("{}.wasm", CONTRACT_MAIN_PURSE);
        ExecuteRequestBuilder::standard(
            DEFAULT_ACCOUNT_ADDR,
            &contract_name,
            (default_account.purse_id(),),
        )
    };

    builder
        .exec_with_exec_request(exec_request)
        .expect_success()
        .commit();
}

#[ignore]
#[test]
fn should_run_main_purse_contract_account_1() {
    let mut builder = InMemoryWasmTestBuilder::default();

    let exec_request_1 = {
        let contract_name = format!("{}.wasm", CONTRACT_TRANSFER_PURSE_TO_ACCOUNT);
        ExecuteRequestBuilder::standard(
            DEFAULT_ACCOUNT_ADDR,
            &contract_name,
            (ACCOUNT_1_ADDR, *DEFAULT_PAYMENT),
        )
    };

    let builder = builder
        .run_genesis(&DEFAULT_GENESIS_CONFIG)
        .exec_with_exec_request(exec_request_1)
        .expect_success()
        .commit();

    let account_1 = builder
        .get_account(ACCOUNT_1_ADDR)
        .expect("should get account");

    let exec_request_2 = {
        let contract_name = format!("{}.wasm", CONTRACT_MAIN_PURSE);
        ExecuteRequestBuilder::standard(ACCOUNT_1_ADDR, &contract_name, (account_1.purse_id(),))
    };

    builder
        .exec_with_exec_request(exec_request_2)
        .expect_success()
        .commit();
}
