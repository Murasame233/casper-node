#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;

use casper_contract::contract_api::{runtime, storage};
use casper_types::{
    contracts::ContractHash, runtime_args, AddressableEntityHash, CLType, CLTyped,
    EntityEntryPoint, EntryPointAccess, EntryPointPayment, EntryPointType, EntryPoints, Key,
    Parameter,
};

const RECURSE_ENTRYPOINT: &str = "recurse";
const ARG_TARGET: &str = "target";
const CONTRACT_HASH_NAME: &str = "regression-contract-hash";

#[no_mangle]
pub extern "C" fn call() {
    let mut entry_points = EntryPoints::new();
    entry_points.add_entry_point(EntityEntryPoint::new(
        RECURSE_ENTRYPOINT,
        vec![Parameter::new(ARG_TARGET, AddressableEntityHash::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Called,
        EntryPointPayment::Caller,
    ));

    let (contract_hash, _contract_version) =
        storage::new_locked_contract(entry_points, None, None, None, None);

    runtime::put_key(
        CONTRACT_HASH_NAME,
        Key::contract_entity_key(AddressableEntityHash::new(contract_hash.value())),
    );
}

#[no_mangle]
pub extern "C" fn recurse() {
    let target: AddressableEntityHash = runtime::get_named_arg(ARG_TARGET);
    runtime::call_contract(
        ContractHash::new(target.value()),
        RECURSE_ENTRYPOINT,
        runtime_args! { ARG_TARGET => target },
    )
}
