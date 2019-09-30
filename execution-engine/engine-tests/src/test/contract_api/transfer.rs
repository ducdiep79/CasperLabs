use std::collections::HashMap;

use grpc::RequestOptions;

use contract_ffi::bytesrepr::ToBytes;
use contract_ffi::key::Key;
use contract_ffi::uref::URef;
use contract_ffi::value::account::{PublicKey, PurseId};
use contract_ffi::value::{Value, U512};
use engine_core::engine_state::MAX_PAYMENT;
use engine_core::engine_state::{EngineConfig, EngineState};
use engine_core::engine_state::{CONV_RATE, SYSTEM_ACCOUNT_ADDR};
use engine_core::execution::MINT_NAME;
use engine_grpc_server::engine_server::ipc_grpc::ExecutionEngineService;
use engine_shared::motes::Motes;
use engine_shared::transform::Transform;
use engine_storage::global_state::in_memory::InMemoryGlobalState;

use crate::support::test_support::{
    self, get_account, DEFAULT_BLOCK_TIME, STANDARD_PAYMENT_CONTRACT,
};
use crate::test::{DEFAULT_ACCOUNT_ADDR, DEFAULT_GENESIS_CONFIG};

const INITIAL_GENESIS_AMOUNT: u64 = 100_000_000_000;

const TRANSFER_1_AMOUNT: u64 = (MAX_PAYMENT * 5) + 1000;
const TRANSFER_2_AMOUNT: u32 = 750;

const TRANSFER_2_AMOUNT_WITH_ADV: u64 = MAX_PAYMENT + TRANSFER_2_AMOUNT as u64;
const TRANSFER_TOO_MUCH: u64 = u64::max_value();

const ACCOUNT_1_ADDR: [u8; 32] = [1u8; 32];
const ACCOUNT_2_ADDR: [u8; 32] = [2u8; 32];
const ACCOUNT_1_INITIAL_BALANCE: u64 = MAX_PAYMENT;

struct TestContext {
    mint_contract_uref: URef,
    locals: HashMap<PurseId, Key>,
}

impl TestContext {
    fn new(mint_contract_uref: URef) -> Self {
        TestContext {
            mint_contract_uref,
            locals: Default::default(),
        }
    }

    /// This method stores an association between a given purse_id and the
    /// the underlying balance uref associated with that purse id.  The balance
    /// uref is extracted from a given set of write transformations, using
    /// the local key generated by the mint contract's uref and the purse
    /// id.
    fn track(&mut self, transforms: &HashMap<Key, Transform>, purse_id: PurseId) {
        let local = {
            let purse_id_bytes = purse_id
                .value()
                .addr()
                .to_bytes()
                .expect("should serialize");
            Key::local(self.mint_contract_uref.addr(), &purse_id_bytes)
        };
        if let Some(Transform::Write(Value::Key(key @ Key::URef(_)))) = transforms.get(&local) {
            self.locals.insert(purse_id, key.normalize());
        }
    }

    fn lookup(&self, transforms: &HashMap<Key, Transform>, purse_id: PurseId) -> Option<Transform> {
        self.locals
            .get(&purse_id)
            .and_then(|local: &Key| transforms.get(local))
            .map(ToOwned::to_owned)
    }
}

#[ignore]
#[test]
fn should_transfer_to_account() {
    let initial_genesis_amount: U512 = U512::from(INITIAL_GENESIS_AMOUNT);
    let transfer_amount: U512 = U512::from(TRANSFER_1_AMOUNT);
    let default_account_key = Key::Account(DEFAULT_ACCOUNT_ADDR);
    let account_key = Key::Account(ACCOUNT_1_ADDR);

    let engine_config = EngineConfig::new().set_use_payment_code(true);
    let global_state = InMemoryGlobalState::empty().unwrap();
    let engine_state = EngineState::new(global_state, engine_config);

    // Run genesis

    let genesis_response = engine_state
        .run_genesis_with_chainspec(RequestOptions::new(), DEFAULT_GENESIS_CONFIG.clone().into())
        .wait_drop_metadata()
        .unwrap();

    let genesis_hash = genesis_response.get_success().get_poststate_hash();

    let genesis_transforms =
        crate::support::test_support::get_genesis_transforms(&genesis_response);

    let system_account = get_account(&genesis_transforms, &Key::Account(SYSTEM_ACCOUNT_ADDR))
        .expect("Unable to get system account");

    let named_keys = system_account.named_keys();

    let mint_contract_uref = named_keys
        .get(MINT_NAME)
        .and_then(Key::as_uref)
        .cloned()
        .expect("Unable to get mint contract URef");

    let mut test_context = TestContext::new(mint_contract_uref);

    let default_account =
        crate::support::test_support::get_account(&genesis_transforms, &default_account_key)
            .expect("should get account");

    let default_account_purse_id = default_account.purse_id();

    test_context.track(&genesis_transforms, default_account_purse_id);

    // Check genesis account balance

    let genesis_balance_transform = test_context
        .lookup(&genesis_transforms, default_account_purse_id)
        .expect("should lookup");

    assert_eq!(
        genesis_balance_transform,
        Transform::Write(Value::UInt512(initial_genesis_amount))
    );

    // Exec transfer contract

    let exec_request = crate::support::test_support::create_exec_request(
        DEFAULT_ACCOUNT_ADDR,
        STANDARD_PAYMENT_CONTRACT,
        (U512::from(MAX_PAYMENT),),
        "transfer_to_account_01.wasm",
        (ACCOUNT_1_ADDR,),
        genesis_hash,
        DEFAULT_BLOCK_TIME,
        [1u8; 32],
        vec![PublicKey::new(DEFAULT_ACCOUNT_ADDR)],
    );

    let exec_response = engine_state
        .exec(RequestOptions::new(), exec_request)
        .wait_drop_metadata()
        .unwrap();

    let exec_transforms = &test_support::get_exec_transforms(&exec_response)[0];

    let account =
        test_support::get_account(&exec_transforms, &account_key).expect("should get account");

    let account_purse_id = account.purse_id();

    test_context.track(&exec_transforms, account_purse_id);

    // Check genesis account balance

    let genesis_balance_transform = test_context
        .lookup(&exec_transforms, default_account_purse_id)
        .expect("should lookup");

    let gas_cost = Motes::from_gas(test_support::get_exec_costs(&exec_response)[0], CONV_RATE)
        .expect("should convert");

    assert_eq!(
        genesis_balance_transform,
        Transform::Write(Value::UInt512(
            initial_genesis_amount - gas_cost.value() - transfer_amount
        ))
    );

    // Check account 1 balance

    let account_1_balance_transform = test_context
        .lookup(&exec_transforms, account_purse_id)
        .expect("should lookup");

    assert_eq!(
        account_1_balance_transform,
        Transform::Write(Value::UInt512(transfer_amount))
    );
}

#[ignore]
#[test]
fn should_transfer_from_account_to_account() {
    let initial_genesis_amount: U512 = U512::from(INITIAL_GENESIS_AMOUNT);
    let transfer_1_amount: U512 = U512::from(TRANSFER_1_AMOUNT);
    let transfer_2_amount: U512 = U512::from(TRANSFER_2_AMOUNT);
    let default_account_key = Key::Account(DEFAULT_ACCOUNT_ADDR);
    let account_1_key = Key::Account(ACCOUNT_1_ADDR);
    let account_2_key = Key::Account(ACCOUNT_2_ADDR);

    let engine_config = EngineConfig::new().set_use_payment_code(true);
    let global_state = InMemoryGlobalState::empty().unwrap();
    let engine_state = EngineState::new(global_state, engine_config);

    // Run genesis

    let genesis_response = engine_state
        .run_genesis_with_chainspec(RequestOptions::new(), DEFAULT_GENESIS_CONFIG.clone().into())
        .wait_drop_metadata()
        .unwrap();

    let genesis_hash = genesis_response.get_success().get_poststate_hash();

    let genesis_transforms = test_support::get_genesis_transforms(&genesis_response);

    let system_account = get_account(&genesis_transforms, &Key::Account(SYSTEM_ACCOUNT_ADDR))
        .expect("Unable to get system account");

    let named_keys = system_account.named_keys();

    let mint_contract_uref = named_keys
        .get(MINT_NAME)
        .and_then(Key::as_uref)
        .cloned()
        .expect("Unable to get mint contract URef");

    let mut test_context = TestContext::new(mint_contract_uref);

    let default_account = test_support::get_account(&genesis_transforms, &default_account_key)
        .expect("should get account");

    let default_account_purse_id = default_account.purse_id();

    test_context.track(&genesis_transforms, default_account_purse_id);

    // Exec transfer 1 contract

    let exec_request = test_support::create_exec_request(
        DEFAULT_ACCOUNT_ADDR,
        STANDARD_PAYMENT_CONTRACT,
        (U512::from(MAX_PAYMENT),),
        "transfer_to_account_01.wasm",
        (ACCOUNT_1_ADDR,),
        genesis_hash,
        DEFAULT_BLOCK_TIME,
        [1u8; 32],
        vec![PublicKey::new(DEFAULT_ACCOUNT_ADDR)],
    );

    let exec_1_response = engine_state
        .exec(RequestOptions::new(), exec_request)
        .wait_drop_metadata()
        .unwrap();

    let exec_1_transforms = &test_support::get_exec_transforms(&exec_1_response)[0];

    let account_1 =
        test_support::get_account(&exec_1_transforms, &account_1_key).expect("should get account");

    let account_1_purse_id = account_1.purse_id();

    test_context.track(&exec_1_transforms, account_1_purse_id);

    // Check genesis account balance

    let genesis_balance_transform = test_context
        .lookup(&exec_1_transforms, default_account_purse_id)
        .expect("should lookup");

    let gas_cost = Motes::from_gas(test_support::get_exec_costs(&exec_1_response)[0], CONV_RATE)
        .expect("should convert");

    assert_eq!(
        genesis_balance_transform,
        Transform::Write(Value::UInt512(
            initial_genesis_amount - gas_cost.value() - transfer_1_amount
        ))
    );

    // Check account 1 balance

    let account_1_balance_transform = test_context
        .lookup(&exec_1_transforms, account_1_purse_id)
        .expect("should lookup");

    assert_eq!(
        account_1_balance_transform,
        Transform::Write(Value::UInt512(transfer_1_amount))
    );

    // Commit transfer contract

    let commit_request = test_support::create_commit_request(genesis_hash, &exec_1_transforms);

    let commit_response = engine_state
        .commit(RequestOptions::new(), commit_request)
        .wait_drop_metadata()
        .unwrap();

    assert!(
        commit_response.has_success(),
        "Commit wasn't successful: {:?}",
        commit_response
    );

    let commit_hash = commit_response.get_success().get_poststate_hash();

    // Exec transfer 2 contract

    let exec_request = test_support::create_exec_request(
        ACCOUNT_1_ADDR,
        STANDARD_PAYMENT_CONTRACT,
        (U512::from(MAX_PAYMENT),),
        "transfer_to_account_02.wasm",
        (U512::from(TRANSFER_2_AMOUNT),),
        commit_hash,
        DEFAULT_BLOCK_TIME,
        [2u8; 32],
        vec![PublicKey::new(ACCOUNT_1_ADDR)],
    );

    let exec_2_response = engine_state
        .exec(RequestOptions::new(), exec_request)
        .wait_drop_metadata()
        .unwrap();

    let exec_2_transforms = &test_support::get_exec_transforms(&exec_2_response)[0];

    let account_2 =
        test_support::get_account(&exec_2_transforms, &account_2_key).expect("should get account");

    let account_2_purse_id = account_2.purse_id();

    test_context.track(&exec_2_transforms, account_2_purse_id);

    // Check account 1 balance

    let account_1_balance_transform = test_context
        .lookup(&exec_2_transforms, account_1_purse_id)
        .expect("should lookup");

    let gas_cost = Motes::from_gas(test_support::get_exec_costs(&exec_2_response)[0], CONV_RATE)
        .expect("should convert");

    assert_eq!(
        account_1_balance_transform,
        Transform::Write(Value::UInt512(
            transfer_1_amount - gas_cost.value() - transfer_2_amount
        ))
    );

    let account_2_balance_transform = test_context
        .lookup(&exec_2_transforms, account_2_purse_id)
        .expect("should lookup");

    assert_eq!(
        account_2_balance_transform,
        Transform::Write(Value::UInt512(transfer_2_amount))
    );
}

#[ignore]
#[test]
fn should_transfer_to_existing_account() {
    let initial_genesis_amount: U512 = U512::from(INITIAL_GENESIS_AMOUNT);
    let transfer_1_amount: U512 = U512::from(TRANSFER_1_AMOUNT);
    let transfer_2_amount: U512 = U512::from(TRANSFER_2_AMOUNT);
    let default_account_key = Key::Account(DEFAULT_ACCOUNT_ADDR);
    let account_1_key = Key::Account(ACCOUNT_1_ADDR);
    let account_2_key = Key::Account(ACCOUNT_2_ADDR);

    let engine_config = EngineConfig::new().set_use_payment_code(true);
    let global_state = InMemoryGlobalState::empty().unwrap();
    let engine_state = EngineState::new(global_state, engine_config);

    // Run genesis

    let genesis_response = engine_state
        .run_genesis_with_chainspec(RequestOptions::new(), DEFAULT_GENESIS_CONFIG.clone().into())
        .wait_drop_metadata()
        .unwrap();

    let genesis_hash = genesis_response.get_success().get_poststate_hash();

    let genesis_transforms = test_support::get_genesis_transforms(&genesis_response);

    let system_account = get_account(&genesis_transforms, &Key::Account(SYSTEM_ACCOUNT_ADDR))
        .expect("Unable to get system account");

    let named_keys = system_account.named_keys();

    let mint_contract_uref = named_keys
        .get(MINT_NAME)
        .and_then(Key::as_uref)
        .cloned()
        .expect("Unable to get mint contract URef");

    let mut test_context = TestContext::new(mint_contract_uref);

    let default_account = test_support::get_account(&genesis_transforms, &default_account_key)
        .expect("should get account");

    let default_account_purse_id = default_account.purse_id();

    test_context.track(&genesis_transforms, default_account_purse_id);

    // Check genesis account balance

    let genesis_balance_transform = test_context
        .lookup(&genesis_transforms, default_account_purse_id)
        .expect("should lookup");

    assert_eq!(
        genesis_balance_transform,
        Transform::Write(Value::UInt512(initial_genesis_amount))
    );

    // Exec transfer contract

    let exec_request = test_support::create_exec_request(
        DEFAULT_ACCOUNT_ADDR,
        STANDARD_PAYMENT_CONTRACT,
        (U512::from(MAX_PAYMENT),),
        "transfer_to_account_01.wasm",
        (ACCOUNT_1_ADDR,),
        genesis_hash,
        DEFAULT_BLOCK_TIME,
        [1u8; 32],
        vec![PublicKey::new(DEFAULT_ACCOUNT_ADDR)],
    );

    let exec_response = engine_state
        .exec(RequestOptions::new(), exec_request)
        .wait_drop_metadata()
        .unwrap();

    let exec_1_transforms = &test_support::get_exec_transforms(&exec_response)[0];

    let account_1 =
        test_support::get_account(&exec_1_transforms, &account_1_key).expect("should get account");

    let account_1_purse_id = account_1.purse_id();

    test_context.track(&exec_1_transforms, account_1_purse_id);

    // Check genesis account balance

    let genesis_balance_transform = test_context
        .lookup(&exec_1_transforms, default_account_purse_id)
        .expect("should lookup");

    let gas_cost = Motes::from_gas(test_support::get_exec_costs(&exec_response)[0], CONV_RATE)
        .expect("should convert");

    assert_eq!(
        genesis_balance_transform,
        Transform::Write(Value::UInt512(
            initial_genesis_amount - gas_cost.value() - transfer_1_amount
        ))
    );

    // Check account 1 balance

    let account_1_balance_transform = test_context
        .lookup(&exec_1_transforms, account_1_purse_id)
        .expect("should lookup");

    assert_eq!(
        account_1_balance_transform,
        Transform::Write(Value::UInt512(transfer_1_amount))
    );

    // Commit transfer contract

    let commit_request = test_support::create_commit_request(genesis_hash, &exec_1_transforms);

    let commit_response = engine_state
        .commit(RequestOptions::new(), commit_request)
        .wait_drop_metadata()
        .unwrap();

    assert!(
        commit_response.has_success(),
        "Commit wasn't successful: {:?}",
        commit_response
    );

    let commit_hash = commit_response.get_success().get_poststate_hash();

    // Exec transfer contract

    let exec_request = test_support::create_exec_request(
        ACCOUNT_1_ADDR,
        STANDARD_PAYMENT_CONTRACT,
        (U512::from(MAX_PAYMENT),),
        "transfer_to_account_02.wasm",
        (U512::from(TRANSFER_2_AMOUNT),),
        commit_hash,
        DEFAULT_BLOCK_TIME,
        [2u8; 32],
        vec![PublicKey::new(ACCOUNT_1_ADDR)],
    );

    let exec_response = engine_state
        .exec(RequestOptions::new(), exec_request)
        .wait_drop_metadata()
        .unwrap();

    let exec_2_transforms = &test_support::get_exec_transforms(&exec_response)[0];

    let account_2 =
        test_support::get_account(&exec_2_transforms, &account_2_key).expect("should get account");

    let account_2_purse_id = account_2.purse_id();

    test_context.track(&exec_2_transforms, account_2_purse_id);

    // Check account 1 balance

    let account_1_balance_transform = test_context
        .lookup(&exec_2_transforms, account_1_purse_id)
        .expect("should lookup");

    let gas_cost = Motes::from_gas(test_support::get_exec_costs(&exec_response)[0], CONV_RATE)
        .expect("should convert");

    assert_eq!(
        account_1_balance_transform,
        Transform::Write(Value::UInt512(
            transfer_1_amount - gas_cost.value() - transfer_2_amount
        ))
    );

    // Check account 2 balance

    let account_2_balance_transform = test_context
        .lookup(&exec_2_transforms, account_2_purse_id)
        .expect("should lookup");

    assert_eq!(
        account_2_balance_transform,
        Transform::Write(Value::UInt512(transfer_2_amount))
    );
}

#[ignore]
#[test]
fn should_fail_when_insufficient_funds() {
    let engine_config = EngineConfig::new().set_use_payment_code(true);
    let global_state = InMemoryGlobalState::empty().unwrap();
    let engine_state = EngineState::new(global_state, engine_config);

    // Run genesis

    let genesis_response = engine_state
        .run_genesis_with_chainspec(RequestOptions::new(), DEFAULT_GENESIS_CONFIG.clone().into())
        .wait_drop_metadata()
        .unwrap();

    let genesis_hash = genesis_response.get_success().get_poststate_hash();

    // Exec transfer contract

    let exec_request = crate::support::test_support::create_exec_request(
        DEFAULT_ACCOUNT_ADDR,
        STANDARD_PAYMENT_CONTRACT,
        (U512::from(MAX_PAYMENT),),
        "transfer_to_account_01.wasm",
        (ACCOUNT_1_ADDR,),
        genesis_hash,
        DEFAULT_BLOCK_TIME,
        [1u8; 32],
        vec![PublicKey::new(DEFAULT_ACCOUNT_ADDR)],
    );

    let exec_response = engine_state
        .exec(RequestOptions::new(), exec_request)
        .wait_drop_metadata()
        .unwrap();

    let exec_1_transforms = &test_support::get_exec_transforms(&exec_response)[0];

    // Commit transfer contract

    let commit_request =
        crate::support::test_support::create_commit_request(genesis_hash, &exec_1_transforms);

    let commit_response = engine_state
        .commit(RequestOptions::new(), commit_request)
        .wait_drop_metadata()
        .unwrap();

    assert!(
        commit_response.has_success(),
        "Commit wasn't successful: {:?}",
        commit_response
    );

    let commit_hash = commit_response.get_success().get_poststate_hash();

    // Exec transfer contract

    let exec_request = crate::support::test_support::create_exec_request(
        ACCOUNT_1_ADDR,
        STANDARD_PAYMENT_CONTRACT,
        (U512::from(MAX_PAYMENT),),
        "transfer_to_account_02.wasm",
        (U512::from(TRANSFER_2_AMOUNT_WITH_ADV),),
        commit_hash,
        DEFAULT_BLOCK_TIME,
        [2u8; 32],
        vec![PublicKey::new(ACCOUNT_1_ADDR)],
    );

    let exec_response = engine_state
        .exec(RequestOptions::new(), exec_request)
        .wait_drop_metadata()
        .unwrap();

    let exec_2_transforms = &test_support::get_exec_transforms(&exec_response)[0];

    // Commit transfer contract

    let commit_request =
        crate::support::test_support::create_commit_request(commit_hash, &exec_2_transforms);

    let commit_response = engine_state
        .commit(RequestOptions::new(), commit_request)
        .wait_drop_metadata()
        .unwrap();

    let commit_hash = commit_response.get_success().get_poststate_hash();

    // Exec transfer contract

    let exec_request = crate::support::test_support::create_exec_request(
        ACCOUNT_1_ADDR,
        STANDARD_PAYMENT_CONTRACT,
        (U512::from(MAX_PAYMENT),),
        "transfer_to_account_02.wasm",
        (U512::from(TRANSFER_TOO_MUCH),),
        commit_hash,
        DEFAULT_BLOCK_TIME,
        [3u8; 32],
        vec![PublicKey::new(ACCOUNT_1_ADDR)],
    );

    let exec_response = engine_state
        .exec(RequestOptions::new(), exec_request)
        .wait_drop_metadata()
        .unwrap();

    assert_eq!(
        "Trap(Trap { kind: Unreachable })",
        exec_response
            .get_success()
            .get_deploy_results()
            .get(0)
            .unwrap()
            .get_execution_result()
            .get_error()
            .get_exec_error()
            .get_message()
    )
}

#[ignore]
#[test]
fn should_transfer_total_amount() {
    let mut builder = test_support::InMemoryWasmTestBuilder::default();

    builder
        .run_genesis(&DEFAULT_GENESIS_CONFIG)
        .exec_with_args(
            DEFAULT_ACCOUNT_ADDR,
            STANDARD_PAYMENT_CONTRACT,
            (U512::from(MAX_PAYMENT),),
            "transfer_purse_to_account.wasm",
            (ACCOUNT_1_ADDR, U512::from(ACCOUNT_1_INITIAL_BALANCE)),
            DEFAULT_BLOCK_TIME,
            [1u8; 32],
        )
        .expect_success()
        .commit()
        .exec_with_args(
            ACCOUNT_1_ADDR,
            STANDARD_PAYMENT_CONTRACT,
            (U512::from(MAX_PAYMENT),),
            // New account transfers exactly N motes to new account (total amount)
            "transfer_purse_to_account.wasm",
            (ACCOUNT_2_ADDR, U512::from(ACCOUNT_1_INITIAL_BALANCE)),
            DEFAULT_BLOCK_TIME,
            [2u8; 32],
        )
        .commit()
        .expect_success()
        .finish();
}
