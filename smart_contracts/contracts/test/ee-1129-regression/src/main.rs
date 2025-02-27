#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::ToString;

use casper_contract::contract_api::{runtime, storage, system};
use casper_types::{
    addressable_entity::Parameters, AddressableEntityHash, CLType, EntityEntryPoint,
    EntryPointAccess, EntryPointPayment, EntryPointType, EntryPoints, Key,
};

const ENTRY_POINT_NAME: &str = "create_purse";
const CONTRACT_KEY: &str = "contract";
const ACCESS_KEY: &str = "access";
const CONTRACT_PACKAGE_KEY: &str = "contract_package";

#[no_mangle]
pub extern "C" fn create_purse() {
    // This should exercise common issues with unsafe providers in mint: new_uref, dictionary_put
    // and put_key.
    let _purse = system::create_purse();
}

#[no_mangle]
pub extern "C" fn call() {
    let entry_points = {
        let mut entry_points = EntryPoints::new();

        let entry_point = EntityEntryPoint::new(
            ENTRY_POINT_NAME,
            Parameters::default(),
            CLType::Unit,
            EntryPointAccess::Public,
            EntryPointType::Called,
            EntryPointPayment::Caller,
        );

        entry_points.add_entry_point(entry_point);

        entry_points
    };

    let (contract_hash, _version) = storage::new_contract(
        entry_points,
        None,
        Some(CONTRACT_PACKAGE_KEY.to_string()),
        Some(ACCESS_KEY.to_string()),
        None,
    );

    runtime::put_key(
        CONTRACT_KEY,
        Key::contract_entity_key(AddressableEntityHash::new(contract_hash.value())),
    );
}
