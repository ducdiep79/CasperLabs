use crate::support::test_support::{ExecuteRequestBuilder, InMemoryWasmTestBuilder};
use contract_ffi::value::account::PublicKey;

use crate::test::{DEFAULT_ACCOUNT_ADDR, DEFAULT_GENESIS_CONFIG};

const CONTRACT_EE_401_REGRESSION: &str = "ee_401_regression";
const CONTRACT_EE_401_REGRESSION_CALL: &str = "ee_401_regression_call";

#[ignore]
#[test]
fn should_execute_contracts_which_provide_extra_urefs() {
    let exec_request_1 = {
        let contract_name = format!("{}.wasm", CONTRACT_EE_401_REGRESSION);
        ExecuteRequestBuilder::standard(DEFAULT_ACCOUNT_ADDR, &contract_name, ())
    };

    let exec_request_2 = {
        let contract_name = format!("{}.wasm", CONTRACT_EE_401_REGRESSION_CALL);
        ExecuteRequestBuilder::standard(
            DEFAULT_ACCOUNT_ADDR,
            &contract_name,
            (PublicKey::new(DEFAULT_ACCOUNT_ADDR),),
        )
    };
    let _result = InMemoryWasmTestBuilder::default()
        .run_genesis(&DEFAULT_GENESIS_CONFIG)
        .exec_with_exec_request(exec_request_1)
        .expect_success()
        .commit()
        .exec_with_exec_request(exec_request_2)
        .expect_success()
        .commit()
        .finish();
}
