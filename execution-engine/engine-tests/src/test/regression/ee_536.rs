use crate::support::test_support::{ExecuteRequestBuilder, InMemoryWasmTestBuilder};
use crate::test::{DEFAULT_ACCOUNT_ADDR, DEFAULT_GENESIS_CONFIG};

const CONTRACT_EE_536_REGRESSION: &str = "ee_536_regression";

#[ignore]
#[test]
fn should_run_ee_536_get_uref_regression_test() {
    // This test runs a contract that's after every call extends the same key with
    // more data
    let exec_request = {
        let contract_name = format!("{}.wasm", CONTRACT_EE_536_REGRESSION);
        ExecuteRequestBuilder::standard(DEFAULT_ACCOUNT_ADDR, &contract_name, ())
    };

    let _result = InMemoryWasmTestBuilder::default()
        .run_genesis(&DEFAULT_GENESIS_CONFIG)
        .exec_with_exec_request(exec_request)
        .expect_success()
        .commit()
        .finish();
}
