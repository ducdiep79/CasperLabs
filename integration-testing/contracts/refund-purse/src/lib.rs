#![no_std]

extern crate contract_ffi;

use contract_ffi::contract_api;
use contract_ffi::contract_api::pointers::{ContractPointer, TURef};
use contract_ffi::key::Key;
use contract_ffi::uref::AccessRights;
use contract_ffi::value::account::PurseId;
use contract_ffi::value::Value;

enum Error {
    GetPosOuterURef = 1000,
    GetPosInnerURef = 1001,
}

fn set_refund_purse(pos: &ContractPointer, p: &PurseId) {
    contract_api::call_contract::<_, ()>(
        pos.clone(),
        &("set_refund_purse", *p),
    );
}

fn get_refund_purse(pos: &ContractPointer) -> Option<PurseId> {
    contract_api::call_contract::<_, Value>(pos.clone(), &("get_refund_purse",))
        .try_deserialize()
        .unwrap()
}

fn get_pos_contract() -> ContractPointer {
    let outer: TURef<Key> = contract_api::get_uref("pos")
        .and_then(Key::to_turef)
        .unwrap_or_else(|| contract_api::revert(Error::GetPosInnerURef as u32));
    if let Some(ContractPointer::URef(inner)) = contract_api::read::<Key>(outer).to_c_ptr() {
        ContractPointer::URef(TURef::new(inner.addr(), AccessRights::READ))
    } else {
        contract_api::revert(Error::GetPosOuterURef as u32)
    }
}

#[no_mangle]
pub extern "C" fn call() {
    let pos_pointer = get_pos_contract();

    let p1 = contract_api::create_purse();
    let p2 = contract_api::create_purse();

    // get_refund_purse should return None before setting it
    let refund_result = get_refund_purse(&pos_pointer);
    if refund_result.is_some() {
        contract_api::revert(1);
    }

    // it should return Some(x) after calling set_refund_purse(x)
    set_refund_purse(&pos_pointer, &p1);
    let refund_purse = match get_refund_purse(&pos_pointer) {
        None => contract_api::revert(2),
        Some(x) if x.value().addr() == p1.value().addr() => x.value(),
        Some(_) => contract_api::revert(3),
    };

    // the returned purse should not have any access rights
    if refund_purse.is_addable() || refund_purse.is_writeable() || refund_purse.is_readable() {
        contract_api::revert(4)
    }

    // get_refund_purse should return correct value after setting a second time
    set_refund_purse(&pos_pointer, &p2);
    match get_refund_purse(&pos_pointer) {
        None => contract_api::revert(5),
        Some(x) if x.value().addr() == p2.value().addr() => (),
        Some(_) => contract_api::revert(6),
    }
}
